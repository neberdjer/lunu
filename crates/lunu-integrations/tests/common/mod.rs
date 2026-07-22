#![allow(dead_code)]

pub mod merge;

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use lunu_core::consts::crypto::SETTINGS_ENCRYPTION_CONTEXT;
use lunu_core::crypto::Encryptor;
use lunu_core::models::Setting;
use lunu_core::repo::SettingsRepo;
use lunu_core::services::SettingsService;

pub struct StubSettings(HashMap<String, String>);

fn setting(key: &str, value: &str) -> Setting {
	Setting {
		key: key.to_string(),
		value: value.to_string(),
		encrypted: false,
		updated_at: Utc::now(),
	}
}

#[async_trait]
impl SettingsRepo for StubSettings {
	async fn get(&self, key: &str) -> lunu_core::Result<Option<Setting>> {
		Ok(self.0.get(key).map(|value| setting(key, value)))
	}
	async fn set(&self, _setting: &Setting) -> lunu_core::Result<()> {
		Ok(())
	}
	async fn get_all(&self) -> lunu_core::Result<Vec<Setting>> {
		Ok(self
			.0
			.iter()
			.map(|(key, value)| setting(key, value))
			.collect())
	}
	async fn delete(&self, _key: &str) -> lunu_core::Result<()> {
		Ok(())
	}
}

fn service(stored: HashMap<String, String>) -> Arc<SettingsService> {
	let encryptor = Encryptor::new("live-test-master-key", SETTINGS_ENCRYPTION_CONTEXT).unwrap();
	Arc::new(SettingsService::new(
		Arc::new(StubSettings(stored)),
		encryptor,
	))
}

/// Settings that hold nothing, so every provider falls back to its registry default.
pub fn no_settings() -> Arc<SettingsService> {
	service(HashMap::new())
}

/// Settings carrying one key read from the environment, for suites that need a live api key.
pub fn settings_from_env(key: &str, variable: &str) -> Arc<SettingsService> {
	let mut stored = HashMap::new();
	if let Ok(value) = std::env::var(variable)
		&& !value.is_empty()
	{
		stored.insert(key.to_string(), value);
	}
	service(stored)
}
