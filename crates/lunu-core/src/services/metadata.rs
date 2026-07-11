use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::consts::metadata::{
	DEFAULT_METADATA_REGION, METADATA_CACHE_TTL_DAYS, METADATA_REGION_SETTING,
	VALID_METADATA_REGIONS,
};
use crate::consts::reasons;
use crate::models::{Book, Chapters, MetadataCacheEntry};
use crate::repo::MetadataCacheRepo;
use crate::services::SettingsService;
use crate::traits::MetadataProvider;
use crate::{Error, Result};

const KIND_SEARCH: &str = "search";
const KIND_BOOK: &str = "book";
const KIND_CHAPTERS: &str = "chapters";

pub struct MetadataService {
	provider: Arc<dyn MetadataProvider>,
	cache: Arc<dyn MetadataCacheRepo>,
	settings: Arc<SettingsService>,
}

impl MetadataService {
	pub fn new(
		provider: Arc<dyn MetadataProvider>,
		cache: Arc<dyn MetadataCacheRepo>,
		settings: Arc<SettingsService>,
	) -> Self {
		Self {
			provider,
			cache,
			settings,
		}
	}

	pub async fn search(&self, query: &str) -> Result<Vec<Book>> {
		let normalized = query.trim().to_lowercase();
		if normalized.is_empty() {
			return Ok(Vec::new());
		}

		let region = self.region().await?;
		let cache_key = format!("{region}:{normalized}");

		if let Some(books) = self
			.read_cache::<Vec<Book>>(KIND_SEARCH, &cache_key)
			.await?
		{
			return Ok(books);
		}

		let books = self.provider.search(query, &region).await?;
		self.write_cache(KIND_SEARCH, &cache_key, &books).await?;
		Ok(books)
	}

	pub async fn get_book(&self, asin: &str) -> Result<Option<Book>> {
		let region = self.region().await?;
		let cache_key = format!("{region}:{asin}");

		if let Some(book) = self.read_cache::<Book>(KIND_BOOK, &cache_key).await? {
			return Ok(Some(book));
		}

		let Some(book) = self.provider.get_book(asin, &region).await? else {
			return Ok(None);
		};

		self.write_cache(KIND_BOOK, &cache_key, &book).await?;
		Ok(Some(book))
	}

	pub async fn get_chapters(&self, asin: &str) -> Result<Option<Chapters>> {
		let region = self.region().await?;
		let cache_key = format!("{region}:{asin}");

		if let Some(chapters) = self
			.read_cache::<Chapters>(KIND_CHAPTERS, &cache_key)
			.await?
		{
			return Ok(Some(chapters));
		}

		let Some(chapters) = self.provider.get_chapters(asin, &region).await? else {
			return Ok(None);
		};

		self.write_cache(KIND_CHAPTERS, &cache_key, &chapters)
			.await?;
		Ok(Some(chapters))
	}

	async fn region(&self) -> Result<String> {
		let region = self
			.settings
			.get(METADATA_REGION_SETTING)
			.await?
			.map(|value| value.trim().to_ascii_lowercase())
			.filter(|value| !value.is_empty())
			.unwrap_or_else(|| DEFAULT_METADATA_REGION.to_string());

		if !VALID_METADATA_REGIONS.contains(&region.as_str()) {
			return Err(Error::Validation(reasons::INVALID_REGION.to_string()));
		}

		Ok(region)
	}

	async fn read_cache<T: DeserializeOwned>(&self, kind: &str, key: &str) -> Result<Option<T>> {
		let Some(entry) = self.cache.get(self.provider.id(), kind, key).await? else {
			return Ok(None);
		};

		if is_stale(entry.fetched_at) {
			return Ok(None);
		}

		let value = serde_json::from_str(&entry.payload)
			.map_err(|error| Error::Internal(format!("corrupt metadata cache: {error}")))?;
		Ok(Some(value))
	}

	async fn write_cache<T: Serialize>(&self, kind: &str, key: &str, value: &T) -> Result<()> {
		let payload = serde_json::to_string(value)
			.map_err(|error| Error::Internal(format!("failed to serialize metadata: {error}")))?;

		self.cache
			.put(&MetadataCacheEntry {
				provider: self.provider.id().to_string(),
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
