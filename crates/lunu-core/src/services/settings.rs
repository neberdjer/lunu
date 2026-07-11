use std::sync::Arc;

use chrono::Utc;

use crate::consts::reasons;
use crate::consts::settings;
use crate::crypto::Encryptor;
use crate::models::Setting;
use crate::repo::SettingsRepo;
use crate::{Error, Result};

pub struct SettingView {
	pub secret: bool,
	pub value: Option<String>,
}

pub struct SettingsService {
	repo: Arc<dyn SettingsRepo>,
	encryptor: Encryptor,
}

impl SettingsService {
	pub fn new(repo: Arc<dyn SettingsRepo>, encryptor: Encryptor) -> Self {
		Self { repo, encryptor }
	}

	pub async fn get(&self, key: &str) -> Result<Option<String>> {
		let Some(setting) = self.repo.get(key).await? else {
			return Ok(None);
		};

		let value = if setting.encrypted {
			self.encryptor.decrypt(&setting.value)?
		} else {
			setting.value
		};

		Ok(Some(value))
	}

	pub async fn view(&self, key: &str) -> Result<Option<SettingView>> {
		let Some(setting) = self.repo.get(key).await? else {
			return Ok(None);
		};

		let value = if setting.encrypted {
			None
		} else {
			Some(setting.value)
		};

		Ok(Some(SettingView {
			secret: setting.encrypted,
			value,
		}))
	}

	pub async fn set(&self, key: &str, value: &str) -> Result<()> {
		let spec = settings::lookup(key)
			.ok_or_else(|| Error::Validation(reasons::UNKNOWN_SETTING.to_string()))?;
		spec.validate(value)
			.map_err(|reason| Error::Validation(reason.to_string()))?;

		let stored = if spec.secret {
			self.encryptor.encrypt(value)?
		} else {
			value.trim().to_string()
		};

		self.repo
			.set(&Setting {
				key: key.to_string(),
				value: stored,
				encrypted: spec.secret,
				updated_at: Utc::now(),
			})
			.await
	}

	pub async fn delete(&self, key: &str) -> Result<()> {
		self.repo.delete(key).await
	}

	pub async fn keys(&self) -> Result<Vec<String>> {
		Ok(self
			.repo
			.get_all()
			.await?
			.into_iter()
			.map(|setting| setting.key)
			.collect())
	}
}
