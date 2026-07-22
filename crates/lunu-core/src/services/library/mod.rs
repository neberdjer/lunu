use std::collections::HashMap;
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

		let mut known = KnownMedia::load(self.media.as_ref()).await?;

		for item in items {
			let existing = self
				.resolve(&mut known, item.asin.as_deref(), &item.abs_item_id)
				.await?;
			match existing {
				Some(media) if media.overridden => summary.skipped += 1,
				Some(media) => {
					let identity = self.identify(Some(&media), &item).await?;
					if newly_matched(media.matched_by, identity.matched_by) {
						summary.matched += 1;
					}
					if !changes(&media, &identity, &item) {
						summary.skipped += 1;
						continue;
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
						..media
					};
					self.media.update(&updated).await?;
					known.remember(updated);
					summary.updated += 1;
				}
				None => {
					let identity = self.identify(None, &item).await?;
					if newly_matched(None, identity.matched_by) {
						summary.matched += 1;
					}
					let fresh = Media {
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
					};
					self.media.insert(&fresh).await?;
					known.remember(fresh);
					summary.imported += 1;
				}
			}
		}

		Ok(summary)
	}

	async fn resolve(
		&self,
		known: &mut KnownMedia,
		asin: Option<&str>,
		abs_item_id: &str,
	) -> Result<Option<Media>> {
		let by_asin = asin.and_then(|asin| known.by_asin(asin));
		let by_abs = known.by_abs(abs_item_id);

		match (by_asin, by_abs) {
			(Some(a), Some(b)) if a.id != b.id && a.overridden && b.overridden => Ok(Some(a)),
			(Some(a), Some(b)) if a.id != b.id => {
				let (keep, drop) = if a.overridden || a.request_id.is_some() {
					(a, b)
				} else {
					(b, a)
				};
				self.media.delete(&drop.id).await?;
				known.forget(&drop);
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

fn changes(media: &Media, identity: &Identity, item: &LibraryItem) -> bool {
	media.work_id != identity.work_id
		|| media.asin != identity.asin
		|| media.matched_by != identity.matched_by
		|| media.abs_item_id.as_deref() != Some(item.abs_item_id.as_str())
		|| media.title != item.title
		|| media.author != item.author
		|| media.cover_url != item.cover_url
		|| media.series_name != item.series_name
		|| media.series_sequence != item.series_sequence
		|| media.library_path != item.library_path
}

struct KnownMedia {
	by_id: HashMap<String, Media>,
	asin_to_id: HashMap<String, String>,
	abs_to_id: HashMap<String, String>,
}

impl KnownMedia {
	async fn load(media: &dyn MediaRepo) -> Result<Self> {
		let rows = media.all().await?;
		let mut known = Self {
			by_id: HashMap::with_capacity(rows.len()),
			asin_to_id: HashMap::new(),
			abs_to_id: HashMap::new(),
		};
		for row in rows {
			known.remember(row);
		}
		Ok(known)
	}

	fn remember(&mut self, media: Media) {
		if let Some(asin) = media.asin.clone() {
			self.asin_to_id.insert(asin, media.id.clone());
		}
		if let Some(abs_item_id) = media.abs_item_id.clone() {
			self.abs_to_id.insert(abs_item_id, media.id.clone());
		}
		self.by_id.insert(media.id.clone(), media);
	}

	fn forget(&mut self, media: &Media) {
		self.by_id.remove(&media.id);
		if let Some(asin) = media.asin.as_deref() {
			self.asin_to_id.remove(asin);
		}
		if let Some(abs_item_id) = media.abs_item_id.as_deref() {
			self.abs_to_id.remove(abs_item_id);
		}
	}

	fn by_asin(&self, asin: &str) -> Option<Media> {
		self.asin_to_id
			.get(asin)
			.and_then(|id| self.by_id.get(id))
			.cloned()
	}

	fn by_abs(&self, abs_item_id: &str) -> Option<Media> {
		self.abs_to_id
			.get(abs_item_id)
			.and_then(|id| self.by_id.get(id))
			.cloned()
	}
}
