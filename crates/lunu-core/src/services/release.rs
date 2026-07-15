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

		let profile = self
			.resolve_profile(request.quality_profile_id.as_deref())
			.await?;
		let releases = self.indexer.search(&request.title).await?;
		let mut ranked = rank_releases(releases, &profile);
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

	pub async fn list_blocklist(&self, request_id: &str) -> Result<Vec<BlocklistEntry>> {
		self.blocklist.list_for_request(request_id).await
	}

	pub async fn remove_blocklist(&self, request_id: &str, entry_id: &str) -> Result<()> {
		if !self.blocklist.remove_by_id(request_id, entry_id).await? {
			return Err(Error::NotFound(format!("blocklist entry {entry_id}")));
		}
		Ok(())
	}

	pub async fn test_indexer(&self) -> Result<()> {
		self.indexer.test_connection().await
	}

	pub async fn search(&self, query: &str) -> Result<Vec<ScoredRelease>> {
		let releases = self.indexer.search(query).await?;
		let profile = self.default_profile().await?;
		Ok(rank_releases(releases, &profile))
	}

	async fn resolve_profile(&self, id: Option<&str>) -> Result<QualityProfile> {
		if let Some(id) = id
			&& let Some(profile) = self.profiles.find_by_id(id).await?
		{
			return Ok(profile);
		}
		self.default_profile().await
	}

	async fn default_profile(&self) -> Result<QualityProfile> {
		Ok(self
			.profiles
			.find_default()
			.await?
			.unwrap_or_else(QualityProfile::builtin_default))
	}
}
