use std::collections::HashSet;
use std::sync::Arc;

use chrono::Utc;

use crate::Result;
use crate::models::{Media, MediaSource, MergeState, Request};
use crate::repo::MediaRepo;
use crate::services::new_id;

pub struct MediaService {
	media: Arc<dyn MediaRepo>,
}

impl MediaService {
	pub fn new(media: Arc<dyn MediaRepo>) -> Self {
		Self { media }
	}

	pub async fn record(&self, request: &Request, library_path: &str) -> Result<String> {
		if let Some(existing) = self.media.find_by_request(&request.id).await? {
			if existing.overridden {
				return Ok(existing.id);
			}
			let updated = Media {
				work_id: Some(request.work_id.clone()),
				format: request.format,
				asin: request.asin.clone(),
				title: request.title.clone(),
				author: request.author.clone(),
				cover_url: request.cover_url.clone(),
				series_name: request.series_name.clone(),
				series_sequence: request.series_sequence.clone(),
				library_path: library_path.to_string(),
				metadata_region: request.metadata_region.clone(),
				..existing
			};
			self.media.update(&updated).await?;
			return Ok(updated.id);
		}
		let media = Media {
			id: new_id(),
			work_id: Some(request.work_id.clone()),
			format: request.format,
			asin: request.asin.clone(),
			abs_item_id: None,
			title: request.title.clone(),
			author: request.author.clone(),
			cover_url: request.cover_url.clone(),
			series_name: request.series_name.clone(),
			series_sequence: request.series_sequence.clone(),
			library_path: library_path.to_string(),
			merged_path: None,
			merge_state: MergeState::default(),
			merge_detail: None,
			merge_backup_path: None,
			source: MediaSource::Request,
			overridden: false,
			matched_by: None,
			metadata_region: request.metadata_region.clone(),
			request_id: Some(request.id.clone()),
			created_at: Utc::now(),
		};
		self.media.upsert_request(&media).await?;
		Ok(media.id)
	}

	pub async fn find(&self, asin: &str) -> Result<Option<Media>> {
		self.media.find_by_asin(asin).await
	}

	pub async fn available_among(&self, asins: &[String]) -> Result<HashSet<String>> {
		Ok(self
			.media
			.available_among(asins)
			.await?
			.into_iter()
			.collect())
	}

	pub async fn count(&self) -> Result<i64> {
		self.media.count().await
	}
}
