use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

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
	cached: RwLock<Option<Arc<HashMap<String, Setting>>>>,
	generation: AtomicU64,
}

impl SettingsService {
	pub fn new(repo: Arc<dyn SettingsRepo>, encryptor: Encryptor) -> Self {
		Self {
			repo,
			encryptor,
			cached: RwLock::new(None),
			generation: AtomicU64::new(0),
		}
	}

	async fn snapshot(&self) -> Result<Arc<HashMap<String, Setting>>> {
		if let Some(cached) = self
			.cached
			.read()
			.expect("settings cache is not poisoned")
			.clone()
		{
			return Ok(cached);
		}
		let generation = self.generation.load(Ordering::Acquire);
		let loaded: HashMap<String, Setting> = self
			.repo
			.get_all()
			.await?
			.into_iter()
			.map(|mut setting| (std::mem::take(&mut setting.key), setting))
			.collect();
		let loaded = Arc::new(loaded);
		let mut guard = self.cached.write().expect("settings cache is not poisoned");
		if self.generation.load(Ordering::Acquire) == generation {
			*guard = Some(loaded.clone());
		}
		Ok(loaded)
	}

	fn invalidate(&self) {
		self.generation.fetch_add(1, Ordering::AcqRel);
		*self.cached.write().expect("settings cache is not poisoned") = None;
	}

	pub async fn get(&self, key: &str) -> Result<Option<String>> {
		let snapshot = self.snapshot().await?;
		let Some(setting) = snapshot.get(key) else {
			return Ok(None);
		};
		Ok(Some(self.plain_value(setting)?))
	}

	fn plain_value(&self, setting: &Setting) -> Result<String> {
		if setting.encrypted {
			self.encryptor.decrypt(&setting.value)
		} else {
			Ok(setting.value.clone())
		}
	}

	pub async fn resolve_many(&self, keys: &[&str]) -> Result<HashMap<String, String>> {
		let stored = self.snapshot().await?;

		let mut resolved = HashMap::with_capacity(keys.len());
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
		let snapshot = self.snapshot().await?;
		let Some(setting) = snapshot.get(key) else {
			return Ok(None);
		};

		let value = if setting.encrypted {
			None
		} else {
			Some(setting.value.clone())
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
			.await?;
		self.invalidate();
		Ok(())
	}

	pub async fn delete(&self, key: &str) -> Result<()> {
		self.reject_broken_pair(key, None).await?;
		self.repo.delete(key).await?;
		self.invalidate();
		Ok(())
	}

	pub async fn keys(&self) -> Result<Vec<String>> {
		let snapshot = self.snapshot().await?;
		let mut keys: Vec<String> = snapshot.keys().cloned().collect();
		keys.sort();
		Ok(keys)
	}
}

fn registry_default(key: &str) -> Option<String> {
	settings::lookup(key)
		.and_then(|spec| spec.default)
		.map(str::to_string)
}
