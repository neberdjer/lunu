use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};

use crate::Result;
use crate::consts::download::{
	MONITOR_MAX_MISSES, MONITOR_MAX_STALLS, MONITOR_POLL_SECS, SETTING_REMOVE_FAILED_DOWNLOADS,
};
use crate::models::{
	Download, DownloadState, DownloadStatus, ImportPayload, JobType, LiveEvent, MonitorPayload,
	Protocol, RequestStatus,
};
use crate::repo::DownloadRepo;
use crate::services::{ClientRoster, JobService, RequestService, SettingsService};
use crate::traits::EventPublisher;

mod content_path;
use content_path::is_safe_content_path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Removal {
	Allowed,
	Forbidden,
}

pub struct MonitorService {
	downloads: Arc<dyn DownloadRepo>,
	clients: ClientRoster,
	requests: Arc<RequestService>,
	jobs: Arc<JobService>,
	events: Arc<dyn EventPublisher>,
	settings: Arc<SettingsService>,
}

impl MonitorService {
	pub fn new(
		downloads: Arc<dyn DownloadRepo>,
		clients: ClientRoster,
		requests: Arc<RequestService>,
		jobs: Arc<JobService>,
		events: Arc<dyn EventPublisher>,
		settings: Arc<SettingsService>,
	) -> Self {
		Self {
			downloads,
			clients,
			requests,
			jobs,
			events,
			settings,
		}
	}

	pub async fn poll(&self, payload: &MonitorPayload) -> Result<()> {
		let Some(download) = self.downloads.find_by_id(&payload.download_id).await? else {
			return Ok(());
		};
		let Some(client_ref) = download.client_ref.as_deref() else {
			return Ok(());
		};
		let Ok(client) = self.clients.by_id(&download.client) else {
			return Ok(());
		};

		let now = Utc::now();
		match client.status(client_ref).await? {
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
			self.fail_download(
				download,
				"download not found in client",
				now,
				Removal::Forbidden,
			)
			.await
		} else {
			self.enqueue_next(&download.request_id, &download.id, attempts, payload.stalls)
				.await
		}
	}

	async fn fail_download(
		&self,
		download: &Download,
		reason: &str,
		now: DateTime<Utc>,
		removal: Removal,
	) -> Result<()> {
		self.downloads
			.update_status(&download.id, DownloadState::Failed, download.progress, now)
			.await?;
		if removal == Removal::Allowed
			&& let Some(client_ref) = download.client_ref.as_deref()
			&& let Ok(client) = self.clients.by_id(&download.client)
			&& self.may_remove(client.protocol(), download.progress).await
		{
			let _ = client.remove(client_ref, true).await;
		}
		if self.is_current(download).await? {
			self.requests
				.mark_failed(&download.request_id, Some(reason))
				.await?;
		}
		Ok(())
	}

	async fn is_current(&self, download: &Download) -> Result<bool> {
		Ok(self
			.downloads
			.find_by_request(&download.request_id)
			.await?
			.is_some_and(|latest| latest.id == download.id))
	}

	async fn may_remove(&self, protocol: Protocol, progress: i64) -> bool {
		if protocol.owes_seeding_at(progress) {
			return false;
		}
		self.settings
			.toggle(SETTING_REMOVE_FAILED_DOWNLOADS)
			.await
			.unwrap_or(false)
	}

	async fn handle_status(
		&self,
		download: &Download,
		payload: &MonitorPayload,
		status: DownloadStatus,
		now: DateTime<Utc>,
	) -> Result<()> {
		let progress = ((status.progress * 100.0).floor() as i64).clamp(0, 100);
		self.downloads
			.update_status(&download.id, status.state, progress, now)
			.await?;

		let mut updated = download.clone();
		updated.state = status.state;
		updated.progress = progress;
		updated.updated_at = now;
		match status.state {
			DownloadState::Completed => {
				self.events.publish(&LiveEvent::Progress(updated));
				self.complete(download, status.content_path, now).await
			}
			DownloadState::Failed => {
				self.events.publish(&LiveEvent::Progress(updated.clone()));
				self.fail_download(&updated, "download failed in client", now, Removal::Allowed)
					.await
			}
			DownloadState::Queued => {
				self.events.publish(&LiveEvent::Progress(updated));
				self.enqueue_next(&download.request_id, &download.id, 0, 0)
					.await
			}
			DownloadState::Downloading => {
				self.events.publish(&LiveEvent::Progress(updated));
				self.handle_progress(download, payload.stalls, progress, now)
					.await
			}
		}
	}

	async fn handle_progress(
		&self,
		download: &Download,
		stalls: i64,
		progress: i64,
		now: DateTime<Utc>,
	) -> Result<()> {
		if progress > download.progress {
			return self
				.enqueue_next(&download.request_id, &download.id, 0, 0)
				.await;
		}

		let stalls = stalls + 1;
		if stalls >= MONITOR_MAX_STALLS {
			self.fail_download(
				download,
				"download stalled with no progress",
				now,
				Removal::Allowed,
			)
			.await
		} else {
			self.enqueue_next(&download.request_id, &download.id, 0, stalls)
				.await
		}
	}

	async fn complete(
		&self,
		download: &Download,
		content_path: Option<String>,
		now: DateTime<Utc>,
	) -> Result<()> {
		let Some(content_path) = content_path else {
			return self
				.fail_download(
					download,
					"download completed but client reported no files",
					now,
					Removal::Allowed,
				)
				.await;
		};
		if !is_safe_content_path(&content_path) {
			return self
				.fail_download(
					download,
					"download completed with an unsafe content path",
					now,
					Removal::Allowed,
				)
				.await;
		}
		if !self.is_current(download).await? {
			return Ok(());
		}
		if let Some(request) = self.requests.get(&download.request_id).await?
			&& request.status == RequestStatus::Available
		{
			return Ok(());
		}
		if self
			.jobs
			.has_active(JobType::Import, &download.request_id)
			.await?
		{
			return Ok(());
		}
		self.requests.mark_importing(&download.request_id).await?;
		let payload = ImportPayload {
			download_id: download.id.clone(),
			content_path,
		};
		self.jobs
			.enqueue_for(JobType::Import, &payload, &download.request_id)
			.await?;
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
