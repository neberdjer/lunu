use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;

use crate::consts::merge;
use crate::consts::reasons;
use crate::consts::settings;
use crate::consts::settings::TOGGLE_ON;
use crate::crypto::Encryptor;
use crate::models::{Setting, SourceDisposition};
use crate::repo::SettingsRepo;
use crate::services::nonempty;
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
		Ok(Some(self.plain_value(&setting)?))
	}

	fn plain_value(&self, setting: &Setting) -> Result<String> {
		if setting.encrypted {
			self.encryptor.decrypt(&setting.value)
		} else {
			Ok(setting.value.clone())
		}
	}

	pub async fn resolve_many(&self, keys: &[&str]) -> Result<HashMap<String, String>> {
		let stored: HashMap<String, Setting> = self
			.repo
			.get_all()
			.await?
			.into_iter()
			.map(|mut setting| (std::mem::take(&mut setting.key), setting))
			.collect();

		let mut resolved = HashMap::new();
		for key in keys {
			let value = match stored.get(*key) {
				Some(setting) => nonempty(Some(self.plain_value(setting)?)),
				None => None,
			}
			.or_else(|| registry_default(key));
			if let Some(value) = value {
				resolved.insert((*key).to_string(), value);
			}
		}
		Ok(resolved)
	}

	pub async fn get_or_default(&self, key: &str) -> Result<Option<String>> {
		Ok(nonempty(self.get(key).await?).or_else(|| registry_default(key)))
	}

	async fn reject_broken_pair(&self, key: &str, value: Option<&str>) -> Result<()> {
		let pending = nonempty(value.map(str::to_string));
		let (action, backup) = match key {
			merge::SETTING_MERGE_SOURCE_ACTION => (
				pending.or_else(|| registry_default(key)),
				self.get_or_default(merge::SETTING_MERGE_BACKUP_DIR).await?,
			),
			merge::SETTING_MERGE_BACKUP_DIR => (
				self.get_or_default(merge::SETTING_MERGE_SOURCE_ACTION)
					.await?,
				pending,
			),
			_ => return Ok(()),
		};
		SourceDisposition::resolve(action.as_deref().unwrap_or_default(), backup).map(|_| ())
	}

	pub async fn toggle(&self, key: &str) -> Result<bool> {
		Ok(self.get_or_default(key).await?.as_deref() == Some(TOGGLE_ON))
	}

	pub async fn number(&self, key: &str) -> Result<Option<i64>> {
		Ok(self
			.get_or_default(key)
			.await?
			.and_then(|value| value.trim().parse().ok()))
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

		self.reject_broken_pair(key, Some(value)).await?;

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
		self.reject_broken_pair(key, None).await?;
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

fn registry_default(key: &str) -> Option<String> {
	settings::lookup(key)
		.and_then(|spec| spec.default)
		.map(str::to_string)
}
