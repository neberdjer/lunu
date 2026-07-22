use std::sync::Arc;

use chrono::Utc;

use crate::consts::reasons;
use crate::helpers::matching::best_match;
use crate::models::{
	Book, ExternalId, Format, LibraryItem, MatchedBy, Media, MediaFilter, MediaSource, MergeState,
};
use crate::repo::MediaRepo;
use crate::services::{MetadataService, WorkService, new_id};
use crate::traits::LibrarySource;
use crate::{Error, Result};

pub struct SyncSummary {
	pub total: usize,
	pub imported: usize,
	pub updated: usize,
	pub skipped: usize,
	pub matched: usize,
}

#[derive(Default)]
struct Identity {
	asin: Option<String>,
	work_id: Option<String>,
	matched_by: Option<MatchedBy>,
}

mod matching;

use matching::newly_matched;

pub struct LibraryService {
	source: Arc<dyn LibrarySource>,
	media: Arc<dyn MediaRepo>,
	metadata: Arc<MetadataService>,
	works: Arc<WorkService>,
}

impl LibraryService {
	pub fn new(
		source: Arc<dyn LibrarySource>,
		media: Arc<dyn MediaRepo>,
		metadata: Arc<MetadataService>,
		works: Arc<WorkService>,
	) -> Self {
		Self {
			source,
			media,
			metadata,
			works,
		}
	}

	pub async fn sync(&self) -> Result<SyncSummary> {
		let items = self.source.list_items().await?;
		let mut summary = SyncSummary {
			total: items.len(),
			imported: 0,
			updated: 0,
			skipped: 0,
			matched: 0,
		};

		for item in items {
			let existing = self
				.resolve(item.asin.as_deref(), &item.abs_item_id)
				.await?;
			match existing {
				Some(media) if media.overridden => summary.skipped += 1,
				Some(media) => {
					let identity = self.identify(Some(&media), &item).await?;
					if newly_matched(media.matched_by, identity.matched_by) {
						summary.matched += 1;
					}
					let updated = Media {
						work_id: identity.work_id,
						asin: identity.asin,
						matched_by: identity.matched_by,
						abs_item_id: Some(item.abs_item_id),
						title: item.title,
						author: item.author,
						cover_url: item.cover_url,
						series_name: item.series_name,
						series_sequence: item.series_sequence,
						library_path: item.library_path,
						..media.clone()
					};
					if updated == media {
						summary.skipped += 1;
						continue;
					}
					self.media.update(&updated).await?;
					summary.updated += 1;
				}
				None => {
					let identity = self.identify(None, &item).await?;
					if newly_matched(None, identity.matched_by) {
						summary.matched += 1;
					}
					self.media
						.insert(&Media {
							id: new_id(),
							work_id: identity.work_id,
							format: Format::Audiobook,
							asin: identity.asin,
							matched_by: identity.matched_by,
							abs_item_id: Some(item.abs_item_id),
							title: item.title,
							author: item.author,
							cover_url: item.cover_url,
							series_name: item.series_name,
							series_sequence: item.series_sequence,
							library_path: item.library_path,
							merged_path: None,
							merge_state: MergeState::default(),
							merge_detail: None,
							merge_backup_path: None,
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
		let by_asin = match asin {
			Some(asin) => self.media.find_by_asin(asin).await?,
			None => None,
		};
		let by_abs = self.media.find_by_abs_item_id(abs_item_id).await?;

		match (by_asin, by_abs) {
			(Some(a), Some(b)) if a.id != b.id && a.overridden && b.overridden => Ok(Some(a)),
			(Some(a), Some(b)) if a.id != b.id => {
				let (keep, drop) = if a.overridden || a.request_id.is_some() {
					(a, b)
				} else {
					(b, a)
				};
				self.media.delete(&drop.id).await?;
				Ok(Some(keep))
			}
			(Some(a), _) => Ok(Some(a)),
			(None, by_abs) => Ok(by_abs),
		}
	}

	pub async fn list(
		&self,
		filter: MediaFilter,
		limit: i64,
		offset: i64,
	) -> Result<(Vec<Media>, i64)> {
		tokio::try_join!(
			self.media.list_page(filter, limit, offset),
			self.media.list_count(filter)
		)
	}

	pub async fn match_asin(&self, media_id: &str, asin: &str) -> Result<Media> {
		let Some(mut media) = self.media.find_by_id(media_id).await? else {
			return Err(Error::NotFound(format!("media {media_id}")));
		};

		let book = self
			.metadata
			.get_book(&ExternalId::asin(asin))
			.await?
			.ok_or_else(|| Error::Validation(reasons::INVALID_ASIN.to_string()))?;

		media.work_id = self.works.for_book(&book).await?;
		media.asin = book.asin().map(str::to_string);
		media.title = book.title;
		media.author = book.authors.into_iter().next();
		media.cover_url = book.cover_url;
		if let Some(series) = book.series.into_iter().next() {
			media.series_name = Some(series.name);
			media.series_sequence = series.position;
		}
		media.overridden = true;
		media.matched_by = Some(MatchedBy::Manual);

		self.media.update(&media).await?;
		Ok(media)
	}
}
