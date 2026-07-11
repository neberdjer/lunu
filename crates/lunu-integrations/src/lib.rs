pub mod download;
pub mod indexer;
pub mod metadata;

pub(crate) mod http;

use std::sync::Arc;

use lunu_core::services::SettingsService;

pub(crate) fn integration_error(error: impl std::fmt::Display) -> lunu_core::Error {
	lunu_core::Error::Integration(error.to_string())
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
