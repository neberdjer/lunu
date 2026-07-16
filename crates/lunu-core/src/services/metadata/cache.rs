use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::MetadataService;
use crate::consts::metadata::METADATA_CACHE_TTL_DAYS;
use crate::models::MetadataCacheEntry;
use crate::{Error, Result};

impl MetadataService {
	pub(super) async fn read_cache<T: DeserializeOwned>(
		&self,
		provider: &str,
		kind: &str,
		key: &str,
	) -> Result<Option<T>> {
		let Some(entry) = self.cache.get(provider, kind, key).await? else {
			return Ok(None);
		};

		if is_stale(entry.fetched_at) {
			return Ok(None);
		}

		let value = serde_json::from_str(&entry.payload)
			.map_err(|error| Error::Internal(format!("corrupt metadata cache: {error}")))?;
		Ok(Some(value))
	}

	pub(super) async fn write_cache<T: Serialize>(
		&self,
		provider: &str,
		kind: &str,
		key: &str,
		value: &T,
	) -> Result<()> {
		let payload = serde_json::to_string(value)
			.map_err(|error| Error::Internal(format!("failed to serialize metadata: {error}")))?;

		self.cache
			.put(&MetadataCacheEntry {
				provider: provider.to_string(),
				kind: kind.to_string(),
				key: key.to_string(),
				payload,
				fetched_at: Utc::now(),
			})
			.await
	}
}

fn is_stale(fetched_at: DateTime<Utc>) -> bool {
	Utc::now() - fetched_at > Duration::days(METADATA_CACHE_TTL_DAYS)
}
