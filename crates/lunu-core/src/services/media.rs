use std::collections::HashSet;
use std::sync::Arc;

use chrono::Utc;

use crate::Result;
use crate::models::{Media, Request};
use crate::repo::MediaRepo;

pub struct MediaService {
	media: Arc<dyn MediaRepo>,
}

impl MediaService {
	pub fn new(media: Arc<dyn MediaRepo>) -> Self {
		Self { media }
	}

	pub async fn record(&self, request: &Request, library_path: &str) -> Result<()> {
		let media = Media {
			asin: request.asin.clone(),
			title: request.title.clone(),
			author: request.author.clone(),
			cover_url: request.cover_url.clone(),
			library_path: library_path.to_string(),
			request_id: Some(request.id.clone()),
			created_at: Utc::now(),
		};
		self.media.upsert(&media).await
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
