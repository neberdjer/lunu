use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};

use crate::Result;
use crate::consts::download::{MONITOR_MAX_MISSES, MONITOR_MAX_STALLS, MONITOR_POLL_SECS};
use crate::models::{
	Download, DownloadState, DownloadStatus, ImportPayload, JobType, LiveEvent, MonitorPayload,
};
use crate::repo::DownloadRepo;
use crate::services::{JobService, RequestService};
use crate::traits::{DownloadClient, EventPublisher};

pub struct MonitorService {
	downloads: Arc<dyn DownloadRepo>,
	client: Arc<dyn DownloadClient>,
	requests: Arc<RequestService>,
	jobs: Arc<JobService>,
	events: Arc<dyn EventPublisher>,
}

impl MonitorService {
	pub fn new(
		downloads: Arc<dyn DownloadRepo>,
		client: Arc<dyn DownloadClient>,
		requests: Arc<RequestService>,
		jobs: Arc<JobService>,
		events: Arc<dyn EventPublisher>,
	) -> Self {
		Self {
			downloads,
			client,
			requests,
			jobs,
			events,
		}
	}

	pub async fn poll(&self, payload: &MonitorPayload) -> Result<()> {
		let Some(download) = self.downloads.find_by_id(&payload.download_id).await? else {
			return Ok(());
		};
		let Some(info_hash) = download.info_hash.as_deref() else {
			return Ok(());
		};

		let now = Utc::now();
		match self.client.status(info_hash).await? {
			None => self.handle_missing(&download, payload, now).await,
			Some(status) => self.handle_status(&download, payload, status, now).await,
		}
	}

	async fn handle_missing(
		&self,
		download: &Download,
		payload: &MonitorPayload,
		now: DateTime<Utc>,
	) -> Result<()> {
		let attempts = payload.misses + 1;
		if attempts >= MONITOR_MAX_MISSES {
			self.downloads
				.update_status(&download.id, DownloadState::Failed, download.progress, now)
				.await?;
			self.requests
				.mark_failed(&download.request_id, Some("download not found in client"))
				.await?;
			Ok(())
		} else {
			self.enqueue_next(&download.request_id, &download.id, attempts, payload.stalls)
				.await
		}
	}

	async fn handle_status(
		&self,
		download: &Download,
		payload: &MonitorPayload,
		status: DownloadStatus,
		now: DateTime<Utc>,
	) -> Result<()> {
		let progress = ((status.progress * 100.0).round() as i64).clamp(0, 100);
		self.downloads
			.update_status(&download.id, status.state, progress, now)
			.await?;

		let mut updated = download.clone();
		updated.state = status.state;
		updated.progress = progress;
		updated.updated_at = now;
		self.events.publish(&LiveEvent::Progress(updated));

		match status.state {
			DownloadState::Completed => self.complete(download, status.content_path).await,
			DownloadState::Failed => self
				.requests
				.mark_failed(&download.request_id, Some("download failed in client"))
				.await
				.map(|_| ()),
			DownloadState::Queued => {
				self.enqueue_next(&download.request_id, &download.id, 0, 0)
					.await
			}
			DownloadState::Downloading => {
				self.handle_progress(download, payload.stalls, progress)
					.await
			}
		}
	}

	async fn handle_progress(&self, download: &Download, stalls: i64, progress: i64) -> Result<()> {
		if progress > download.progress {
			return self
				.enqueue_next(&download.request_id, &download.id, 0, 0)
				.await;
		}

		let stalls = stalls + 1;
		if stalls >= MONITOR_MAX_STALLS {
			self.requests
				.mark_failed(
					&download.request_id,
					Some("download stalled with no progress"),
				)
				.await
				.map(|_| ())
		} else {
			self.enqueue_next(&download.request_id, &download.id, 0, stalls)
				.await
		}
	}

	async fn complete(&self, download: &Download, content_path: Option<String>) -> Result<()> {
		self.requests.mark_importing(&download.request_id).await?;
		if let Some(content_path) = content_path {
			let payload = ImportPayload {
				download_id: download.id.clone(),
				content_path,
			};
			self.jobs
				.enqueue_for(JobType::Import, &payload, &download.request_id)
				.await?;
		}
		Ok(())
	}

	async fn enqueue_next(
		&self,
		request_id: &str,
		download_id: &str,
		misses: i64,
		stalls: i64,
	) -> Result<()> {
		let payload = MonitorPayload {
			download_id: download_id.to_string(),
			misses,
			stalls,
		};
		let run_after = Utc::now() + Duration::seconds(MONITOR_POLL_SECS);
		self.jobs
			.enqueue_for_at(JobType::MonitorDownload, &payload, request_id, run_after)
			.await?;
		Ok(())
	}
}
