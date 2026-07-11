use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};

use crate::Result;
use crate::consts::download::{MONITOR_MAX_MISSES, MONITOR_POLL_SECS};
use crate::models::{
	Download, DownloadState, DownloadStatus, ImportPayload, JobType, MonitorPayload,
};
use crate::repo::DownloadRepo;
use crate::services::{JobService, RequestService};
use crate::traits::DownloadClient;

pub struct MonitorService {
	downloads: Arc<dyn DownloadRepo>,
	client: Arc<dyn DownloadClient>,
	requests: Arc<RequestService>,
	jobs: Arc<JobService>,
}

impl MonitorService {
	pub fn new(
		downloads: Arc<dyn DownloadRepo>,
		client: Arc<dyn DownloadClient>,
		requests: Arc<RequestService>,
		jobs: Arc<JobService>,
	) -> Self {
		Self {
			downloads,
			client,
			requests,
			jobs,
		}
	}

	pub async fn poll(&self, download_id: &str, misses: i64) -> Result<()> {
		let Some(download) = self.downloads.find_by_id(download_id).await? else {
			return Ok(());
		};
		let Some(info_hash) = download.info_hash.as_deref() else {
			return Ok(());
		};

		let now = Utc::now();
		match self.client.status(info_hash).await? {
			None => self.handle_missing(&download, misses, now).await,
			Some(status) => self.handle_status(&download, status, now).await,
		}
	}

	async fn handle_missing(
		&self,
		download: &Download,
		misses: i64,
		now: DateTime<Utc>,
	) -> Result<()> {
		let attempts = misses + 1;
		if attempts >= MONITOR_MAX_MISSES {
			self.downloads
				.update_status(&download.id, DownloadState::Failed, download.progress, now)
				.await?;
			self.requests.mark_failed(&download.request_id).await?;
			Ok(())
		} else {
			self.enqueue_next(&download.id, attempts).await
		}
	}

	async fn handle_status(
		&self,
		download: &Download,
		status: DownloadStatus,
		now: DateTime<Utc>,
	) -> Result<()> {
		let progress = ((status.progress * 100.0).round() as i64).clamp(0, 100);
		self.downloads
			.update_status(&download.id, status.state, progress, now)
			.await?;

		match status.state {
			DownloadState::Completed => self.complete(download, status.content_path).await,
			DownloadState::Failed => self
				.requests
				.mark_failed(&download.request_id)
				.await
				.map(|_| ()),
			DownloadState::Queued | DownloadState::Downloading => {
				self.enqueue_next(&download.id, 0).await
			}
		}
	}

	async fn complete(&self, download: &Download, content_path: Option<String>) -> Result<()> {
		self.requests.mark_importing(&download.request_id).await?;
		if let Some(content_path) = content_path {
			let payload = ImportPayload {
				download_id: download.id.clone(),
				content_path,
			};
			self.jobs.enqueue(JobType::Import, &payload).await?;
		}
		Ok(())
	}

	async fn enqueue_next(&self, download_id: &str, misses: i64) -> Result<()> {
		let payload = MonitorPayload {
			download_id: download_id.to_string(),
			misses,
		};
		let run_after = Utc::now() + Duration::seconds(MONITOR_POLL_SECS);
		self.jobs
			.enqueue_at(JobType::MonitorDownload, &payload, run_after)
			.await?;
		Ok(())
	}
}
