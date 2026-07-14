use std::sync::Arc;

use chrono::Utc;

use crate::consts::reasons;
use crate::models::{Media, MediaSource};
use crate::repo::MediaRepo;
use crate::services::{MetadataService, new_id};
use crate::traits::LibrarySource;
use crate::{Error, Result};

pub struct SyncSummary {
	pub total: usize,
	pub imported: usize,
	pub updated: usize,
	pub skipped: usize,
}

pub struct LibraryService {
	source: Arc<dyn LibrarySource>,
	media: Arc<dyn MediaRepo>,
	metadata: Arc<MetadataService>,
}

impl LibraryService {
	pub fn new(
		source: Arc<dyn LibrarySource>,
		media: Arc<dyn MediaRepo>,
		metadata: Arc<MetadataService>,
	) -> Self {
		Self {
			source,
			media,
			metadata,
		}
	}

	pub async fn sync(&self) -> Result<SyncSummary> {
		let items = self.source.list_items().await?;
		let mut summary = SyncSummary {
			total: items.len(),
			imported: 0,
			updated: 0,
			skipped: 0,
		};

		for item in items {
			let existing = self
				.resolve(item.asin.as_deref(), &item.abs_item_id)
				.await?;
			match existing {
				Some(media) if media.overridden => summary.skipped += 1,
				Some(mut media) => {
					media.asin = item.asin;
					media.abs_item_id = Some(item.abs_item_id);
					media.title = item.title;
					media.author = item.author;
					media.cover_url = item.cover_url;
					media.series_name = item.series_name;
					media.series_sequence = item.series_sequence;
					media.library_path = item.library_path;
					self.media.update(&media).await?;
					summary.updated += 1;
				}
				None => {
					self.media
						.insert(&Media {
							id: new_id(),
							asin: item.asin,
							abs_item_id: Some(item.abs_item_id),
							title: item.title,
							author: item.author,
							cover_url: item.cover_url,
							series_name: item.series_name,
							series_sequence: item.series_sequence,
							library_path: item.library_path,
							source: MediaSource::Abs,
							overridden: false,
							request_id: None,
							created_at: Utc::now(),
						})
						.await?;
					summary.imported += 1;
				}
			}
		}

		Ok(summary)
	}

	async fn resolve(&self, asin: Option<&str>, abs_item_id: &str) -> Result<Option<Media>> {
		if let Some(asin) = asin
			&& let Some(media) = self.media.find_by_asin(asin).await?
		{
			return Ok(Some(media));
		}
		self.media.find_by_abs_item_id(abs_item_id).await
	}

	pub async fn list(
		&self,
		unmatched_only: bool,
		limit: i64,
		offset: i64,
	) -> Result<(Vec<Media>, i64)> {
		let items = self.media.list_page(unmatched_only, limit, offset).await?;
		let total = self.media.list_count(unmatched_only).await?;
		Ok((items, total))
	}

	pub async fn match_asin(&self, media_id: &str, asin: &str) -> Result<Media> {
		let Some(mut media) = self.media.find_by_id(media_id).await? else {
			return Err(Error::NotFound(format!("media {media_id}")));
		};

		let book = self
			.metadata
			.get_book(asin)
			.await?
			.ok_or_else(|| Error::Validation(reasons::INVALID_ASIN.to_string()))?;

		media.asin = Some(book.asin);
		media.title = book.title;
		media.author = book.authors.into_iter().next();
		media.cover_url = book.cover_url;
		if let Some(series) = book.series.into_iter().next() {
			media.series_name = Some(series.name);
			media.series_sequence = series.position;
		}
		media.overridden = true;

		self.media.update(&media).await?;
		Ok(media)
	}
}
