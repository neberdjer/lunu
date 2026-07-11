use std::sync::Arc;

use crate::helpers::scoring::rank_releases;
use crate::models::{QualityProfile, ScoredRelease};
use crate::repo::{QualityProfileRepo, RequestRepo};
use crate::traits::Indexer;
use crate::{Error, Result};

pub struct ReleaseService {
	indexer: Arc<dyn Indexer>,
	profiles: Arc<dyn QualityProfileRepo>,
	requests: Arc<dyn RequestRepo>,
}

impl ReleaseService {
	pub fn new(
		indexer: Arc<dyn Indexer>,
		profiles: Arc<dyn QualityProfileRepo>,
		requests: Arc<dyn RequestRepo>,
	) -> Self {
		Self {
			indexer,
			profiles,
			requests,
		}
	}

	pub async fn for_request(&self, request_id: &str) -> Result<Vec<ScoredRelease>> {
		let request = self
			.requests
			.find_by_id(request_id)
			.await?
			.ok_or_else(|| Error::NotFound(format!("request {request_id}")))?;

		self.search(&request.title).await
	}

	pub async fn search(&self, query: &str) -> Result<Vec<ScoredRelease>> {
		let releases = self.indexer.search(query).await?;
		let profile = self.default_profile().await?;
		Ok(rank_releases(releases, &profile))
	}

	async fn default_profile(&self) -> Result<QualityProfile> {
		Ok(self
			.profiles
			.find_default()
			.await?
			.unwrap_or_else(QualityProfile::builtin_default))
	}
}
