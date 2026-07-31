use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::consts::settings::{
	APPRISE_URL, DISCORD_WEBHOOK_URL, NOTIFICATION_WEBHOOK_URL, SLACK_WEBHOOK_URL,
};
use lunu_core::models::NotificationEvent;
use lunu_core::services::SettingsService;
use lunu_core::traits::Notifier;
use serde_json::{Value, json};

use crate::http::send_write;
use crate::{http_client_builder, integration_error, optional_setting};

const REQUEST_TIMEOUT_SECS: u64 = 15;

pub struct WebhookChannel {
	http: reqwest::Client,
	settings: Arc<SettingsService>,
	id: &'static str,
	setting_key: &'static str,
	body: fn(&NotificationEvent) -> Value,
}

impl WebhookChannel {
	fn build(
		settings: Arc<SettingsService>,
		id: &'static str,
		setting_key: &'static str,
		body: fn(&NotificationEvent) -> Value,
	) -> Self {
		let http = http_client_builder(Duration::from_secs(REQUEST_TIMEOUT_SECS))
			.build()
			.expect("reqwest client builds with static configuration");

		Self {
			http,
			settings,
			id,
			setting_key,
			body,
		}
	}

	pub fn generic(settings: Arc<SettingsService>) -> Self {
		Self::build(settings, "webhook", NOTIFICATION_WEBHOOK_URL, generic_body)
	}

	pub fn discord(settings: Arc<SettingsService>) -> Self {
		Self::build(settings, "discord", DISCORD_WEBHOOK_URL, discord_body)
	}

	pub fn slack(settings: Arc<SettingsService>) -> Self {
		Self::build(settings, "slack", SLACK_WEBHOOK_URL, slack_body)
	}

	pub fn apprise(settings: Arc<SettingsService>) -> Self {
		Self::build(settings, "apprise", APPRISE_URL, apprise_body)
	}
}

fn generic_body(event: &NotificationEvent) -> Value {
	json!({
		"kind": event.kind,
		"request_id": event.request_id,
		"title": event.title,
		"message": event.message(),
	})
}

fn discord_body(event: &NotificationEvent) -> Value {
	json!({ "content": event.message() })
}

fn slack_body(event: &NotificationEvent) -> Value {
	json!({ "text": event.message() })
}

fn apprise_body(event: &NotificationEvent) -> Value {
	json!({ "title": event.kind.summary(), "body": event.message() })
}

#[async_trait]
impl Notifier for WebhookChannel {
	fn id(&self) -> &'static str {
		self.id
	}

	async fn deliver(&self, event: &NotificationEvent) -> Result<()> {
		let Some(url) = optional_setting(&self.settings, self.setting_key).await? else {
			return Ok(());
		};

		let body = (self.body)(event);
		let response = send_write(|| self.http.post(&url).json(&body)).await?;
		response.error_for_status().map_err(integration_error)?;
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use lunu_core::models::NotificationKind;

	use super::*;

	fn event() -> NotificationEvent {
		NotificationEvent {
			kind: NotificationKind::RequestAvailable,
			request_id: "r1".to_string(),
			title: "Dune".to_string(),
			user_id: "u1".to_string(),
		}
	}

	#[test]
	fn generic_body_carries_event_fields_but_not_the_recipient() {
		let value = generic_body(&event());
		assert_eq!(value["kind"], "request-available");
		assert_eq!(value["request_id"], "r1");
		assert_eq!(value["title"], "Dune");
		assert_eq!(value["message"], "Now available: Dune");
		assert!(
			value.get("user_id").is_none(),
			"the recipient's internal id must not leak to an external webhook"
		);
	}

	#[test]
	fn discord_body_uses_content_field() {
		assert_eq!(discord_body(&event())["content"], "Now available: Dune");
	}

	#[test]
	fn slack_body_uses_text_field() {
		assert_eq!(slack_body(&event())["text"], "Now available: Dune");
	}

	#[test]
	fn apprise_body_carries_title_and_body() {
		let value = apprise_body(&event());
		assert_eq!(value["title"], "Now available");
		assert_eq!(value["body"], "Now available: Dune");
	}
}
