use std::collections::HashSet;
use std::sync::Arc;

use chrono::Utc;

use crate::helpers::scoring::rank_releases;
use crate::models::{BlocklistEntry, QualityProfile, ScoredRelease};
use crate::repo::{BlocklistRepo, QualityProfileRepo, RequestRepo};
use crate::services::new_id;
use crate::traits::Indexer;
use crate::{Error, Result};

pub struct ReleaseService {
	indexer: Arc<dyn Indexer>,
	profiles: Arc<dyn QualityProfileRepo>,
	requests: Arc<dyn RequestRepo>,
	blocklist: Arc<dyn BlocklistRepo>,
}

impl ReleaseService {
	pub fn new(
		indexer: Arc<dyn Indexer>,
		profiles: Arc<dyn QualityProfileRepo>,
		requests: Arc<dyn RequestRepo>,
		blocklist: Arc<dyn BlocklistRepo>,
	) -> Self {
		Self {
			indexer,
			profiles,
			requests,
			blocklist,
		}
	}

	pub async fn for_request(&self, request_id: &str) -> Result<Vec<ScoredRelease>> {
		let request = self
			.requests
			.find_by_id(request_id)
			.await?
			.ok_or_else(|| Error::NotFound(format!("request {request_id}")))?;

		let mut ranked = self.search(&request.title).await?;
		let blocked = self.blocklist.urls_for_request(request_id).await?;
		let blocked: HashSet<&str> = blocked.iter().map(String::as_str).collect();
		ranked.retain(|scored| !blocked.contains(scored.release.download_url.as_str()));
		Ok(ranked)
	}

	pub async fn blocklist_release(&self, request_id: &str, download_url: &str) -> Result<()> {
		self.requests
			.find_by_id(request_id)
			.await?
			.ok_or_else(|| Error::NotFound(format!("request {request_id}")))?;
		self.blocklist
			.add(&BlocklistEntry {
				id: new_id(),
				request_id: request_id.to_string(),
				download_url: download_url.to_string(),
				created_at: Utc::now(),
			})
			.await
	}

	pub async fn test_indexer(&self) -> Result<()> {
		self.indexer.test_connection().await
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
