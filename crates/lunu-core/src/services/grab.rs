use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::consts::download::{GRAB_CATEGORY, MONITOR_POLL_SECS};
use crate::consts::reasons;
use crate::helpers::magnet;
use crate::models::{Download, DownloadState, JobType, MonitorPayload};
use crate::repo::DownloadRepo;
use crate::services::{JobService, ReleaseService, RequestService, new_id};
use crate::traits::DownloadClient;
use crate::{Error, Result};

pub struct ReleaseSelection {
	pub title: String,
	pub indexer: String,
	pub download_url: String,
	pub info_hash: Option<String>,
}

impl ReleaseSelection {
	fn resolved_info_hash(&self) -> Option<String> {
		self.info_hash
			.clone()
			.or_else(|| magnet::info_hash(&self.download_url))
	}
}

pub struct GrabService {
	downloads: Arc<dyn DownloadRepo>,
	requests: Arc<RequestService>,
	releases: Arc<ReleaseService>,
	client: Arc<dyn DownloadClient>,
	jobs: Arc<JobService>,
}

impl GrabService {
	pub fn new(
		downloads: Arc<dyn DownloadRepo>,
		requests: Arc<RequestService>,
		releases: Arc<ReleaseService>,
		client: Arc<dyn DownloadClient>,
		jobs: Arc<JobService>,
	) -> Self {
		Self {
			downloads,
			requests,
			releases,
			client,
			jobs,
		}
	}

	pub async fn grab(
		&self,
		request_id: &str,
		selection: Option<ReleaseSelection>,
	) -> Result<Download> {
		if self.requests.get(request_id).await?.is_none() {
			return Err(Error::NotFound(format!("request {request_id}")));
		}

		let selection = match selection {
			Some(selection) => selection,
			None => self.best_release(request_id).await?,
		};

		self.client
			.add(&selection.download_url, GRAB_CATEGORY)
			.await?;

		let now = Utc::now();
		let info_hash = selection.resolved_info_hash();
		let download = Download {
			id: new_id(),
			request_id: request_id.to_string(),
			client: self.client.id().to_string(),
			category: GRAB_CATEGORY.to_string(),
			release_title: selection.title,
			indexer: selection.indexer,
			download_url: selection.download_url,
			info_hash,
			state: DownloadState::Queued,
			progress: 0,
			created_at: now,
			updated_at: now,
		};
		self.downloads.create(&download).await?;

		self.requests.mark_downloading(request_id).await?;

		if download.info_hash.is_some() {
			let payload = MonitorPayload {
				download_id: download.id.clone(),
				misses: 0,
			};
			let run_after = now + Duration::seconds(MONITOR_POLL_SECS);
			self.jobs
				.enqueue_at(JobType::MonitorDownload, &payload, run_after)
				.await?;
		}

		Ok(download)
	}

	pub async fn for_request(&self, request_id: &str) -> Result<Option<Download>> {
		self.downloads.find_by_request(request_id).await
	}

	pub async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<Download>> {
		self.downloads.list_page(limit, offset).await
	}

	pub async fn count(&self) -> Result<i64> {
		self.downloads.count().await
	}

	async fn best_release(&self, request_id: &str) -> Result<ReleaseSelection> {
		let best = self
			.releases
			.for_request(request_id)
			.await?
			.into_iter()
			.next()
			.ok_or_else(|| Error::Validation(reasons::NO_RELEASES.to_string()))?;

		Ok(ReleaseSelection {
			title: best.release.title,
			indexer: best.release.indexer,
			download_url: best.release.download_url,
			info_hash: best.release.info_hash,
		})
	}
}
