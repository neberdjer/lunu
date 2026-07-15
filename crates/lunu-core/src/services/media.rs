use std::collections::HashSet;
use std::sync::Arc;

use chrono::Utc;

use crate::Result;
use crate::models::{Media, MediaSource, Request};
use crate::repo::MediaRepo;
use crate::services::new_id;

pub struct MediaService {
	media: Arc<dyn MediaRepo>,
}

impl MediaService {
	pub fn new(media: Arc<dyn MediaRepo>) -> Self {
		Self { media }
	}

	pub async fn record(&self, request: &Request, library_path: &str) -> Result<()> {
		if let Some(existing) = self.media.find_by_request(&request.id).await? {
			let updated = Media {
				asin: request.asin.clone(),
				title: request.title.clone(),
				author: request.author.clone(),
				cover_url: request.cover_url.clone(),
				library_path: library_path.to_string(),
				..existing
			};
			return self.media.update(&updated).await;
		}
		let media = Media {
			id: new_id(),
			asin: request.asin.clone(),
			abs_item_id: None,
			title: request.title.clone(),
			author: request.author.clone(),
			cover_url: request.cover_url.clone(),
			series_name: None,
			series_sequence: None,
			library_path: library_path.to_string(),
			source: MediaSource::Request,
			overridden: false,
			request_id: Some(request.id.clone()),
			created_at: Utc::now(),
		};
		self.media.upsert_request(&media).await
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
