use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::services::SettingsService;
use lunu_core::traits::{AuthProvider, ExternalIdentity};
use reqwest::StatusCode;
use serde::Deserialize;

use crate::{http_client_builder, integration_error, optional_setting};

const PROVIDER_NAME: &str = "audiobookshelf";
const REQUEST_TIMEOUT_SECS: u64 = 30;
const SETTING_URL: &str = lunu_core::consts::settings::ABS_URL;

pub struct AudiobookshelfProvider {
	http: reqwest::Client,
	settings: Arc<SettingsService>,
}

impl AudiobookshelfProvider {
	pub fn new(settings: Arc<SettingsService>) -> Self {
		let http = http_client_builder(Duration::from_secs(REQUEST_TIMEOUT_SECS))
			.build()
			.expect("reqwest client builds with static configuration");

		Self { http, settings }
	}
}

#[async_trait]
impl AuthProvider for AudiobookshelfProvider {
	fn name(&self) -> &'static str {
		PROVIDER_NAME
	}

	async fn authenticate(
		&self,
		username: &str,
		password: &str,
	) -> Result<Option<ExternalIdentity>> {
		let Some(base_url) = optional_setting(&self.settings, SETTING_URL).await? else {
			return Ok(None);
		};

		let url = format!("{}/login", base_url.trim_end_matches('/'));
		let response = self
			.http
			.post(&url)
			.json(&serde_json::json!({ "username": username, "password": password }))
			.send()
			.await
			.map_err(integration_error)?;

		if matches!(
			response.status(),
			StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
		) {
			return Ok(None);
		}

		let body: LoginResponse = crate::http::json_response(response).await?;
		Ok(Some(ExternalIdentity {
			username: body.user.username,
			email: body.user.email.filter(|email| !email.is_empty()),
		}))
	}
}

#[derive(Deserialize)]
struct LoginResponse {
	user: AbsUser,
}

#[derive(Deserialize)]
struct AbsUser {
	username: String,
	email: Option<String>,
}
