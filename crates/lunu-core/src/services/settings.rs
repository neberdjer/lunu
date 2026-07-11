use std::sync::Arc;

use chrono::Utc;

use crate::Result;
use crate::crypto::Encryptor;
use crate::models::Setting;
use crate::repo::SettingsRepo;

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

	pub async fn set(&self, key: &str, value: &str, secret: bool) -> Result<()> {
		let value = if secret {
			self.encryptor.encrypt(value)?
		} else {
			value.to_string()
		};

		self.repo
			.set(&Setting {
				key: key.to_string(),
				value,
				encrypted: secret,
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
