use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::consts::metadata::METADATA_CACHE_TTL_DAYS;
use crate::models::{Book, Chapters, MetadataCacheEntry};
use crate::repo::MetadataCacheRepo;
use crate::traits::MetadataProvider;
use crate::{Error, Result};

const KIND_SEARCH: &str = "search";
const KIND_BOOK: &str = "book";
const KIND_CHAPTERS: &str = "chapters";

pub struct MetadataService {
	provider: Arc<dyn MetadataProvider>,
	cache: Arc<dyn MetadataCacheRepo>,
}

impl MetadataService {
	pub fn new(provider: Arc<dyn MetadataProvider>, cache: Arc<dyn MetadataCacheRepo>) -> Self {
		Self { provider, cache }
	}

	pub async fn search(&self, query: &str) -> Result<Vec<Book>> {
		let key = query.trim().to_lowercase();
		if key.is_empty() {
			return Ok(Vec::new());
		}

		if let Some(books) = self.read_cache::<Vec<Book>>(KIND_SEARCH, &key).await? {
			return Ok(books);
		}

		let books = self.provider.search(query).await?;
		self.write_cache(KIND_SEARCH, &key, &books).await?;
		Ok(books)
	}

	pub async fn get_book(&self, asin: &str) -> Result<Option<Book>> {
		if let Some(book) = self.read_cache::<Book>(KIND_BOOK, asin).await? {
			return Ok(Some(book));
		}

		let Some(book) = self.provider.get_book(asin).await? else {
			return Ok(None);
		};

		self.write_cache(KIND_BOOK, asin, &book).await?;
		Ok(Some(book))
	}

	pub async fn get_chapters(&self, asin: &str) -> Result<Option<Chapters>> {
		if let Some(chapters) = self.read_cache::<Chapters>(KIND_CHAPTERS, asin).await? {
			return Ok(Some(chapters));
		}

		let Some(chapters) = self.provider.get_chapters(asin).await? else {
			return Ok(None);
		};

		self.write_cache(KIND_CHAPTERS, asin, &chapters).await?;
		Ok(Some(chapters))
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
