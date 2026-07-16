use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::consts::settings::NTFY_TOPIC_URL;
use lunu_core::models::NotificationEvent;
use lunu_core::services::SettingsService;
use lunu_core::traits::Notifier;

use crate::http::send_with_retry;
use crate::{http_client_builder, integration_error, optional_setting};

const REQUEST_TIMEOUT_SECS: u64 = 15;

pub struct NtfyChannel {
	http: reqwest::Client,
	settings: Arc<SettingsService>,
}

impl NtfyChannel {
	pub fn new(settings: Arc<SettingsService>) -> Self {
		let http = http_client_builder(Duration::from_secs(REQUEST_TIMEOUT_SECS))
			.build()
			.expect("reqwest client builds with static configuration");

		Self { http, settings }
	}
}

#[async_trait]
impl Notifier for NtfyChannel {
	fn id(&self) -> &'static str {
		"ntfy"
	}

	async fn deliver(&self, event: &NotificationEvent) -> Result<()> {
		let Some(url) = optional_setting(&self.settings, NTFY_TOPIC_URL).await? else {
			return Ok(());
		};

		let response = send_with_retry(|| {
			self.http
				.post(&url)
				.header("X-Title", event.kind.summary())
				.body(event.message())
		})
		.await?;
		response.error_for_status().map_err(integration_error)?;
		Ok(())
	}
}
