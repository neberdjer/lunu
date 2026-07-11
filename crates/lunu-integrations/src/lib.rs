pub mod auth;
pub mod download;
pub mod indexer;
pub mod library;
pub mod metadata;

pub(crate) mod http;

use std::sync::Arc;
use std::time::Duration;

use lunu_core::services::SettingsService;

pub(crate) fn integration_error(error: impl std::fmt::Display) -> lunu_core::Error {
	lunu_core::Error::Integration(error.to_string())
}

pub(crate) fn http_client_builder(timeout: Duration) -> reqwest::ClientBuilder {
	reqwest::Client::builder()
		.user_agent(concat!("lunu/", env!("CARGO_PKG_VERSION")))
		.timeout(timeout)
}

pub(crate) async fn optional_setting(
	settings: &Arc<SettingsService>,
	key: &str,
) -> lunu_core::Result<Option<String>> {
	Ok(settings
		.get(key)
		.await?
		.map(|value| value.trim().to_string())
		.filter(|value| !value.is_empty()))
}

pub(crate) async fn required_setting(
	settings: &Arc<SettingsService>,
	key: &str,
	not_configured: &str,
) -> lunu_core::Result<String> {
	optional_setting(settings, key)
		.await?
		.ok_or_else(|| lunu_core::Error::Validation(not_configured.to_string()))
}
