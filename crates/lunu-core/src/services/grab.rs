use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::consts::download::{GRAB_CATEGORY, MONITOR_POLL_SECS};
use crate::consts::reasons;
use crate::helpers::magnet;
use crate::models::{Download, DownloadState, JobType, MonitorPayload, Protocol};
use crate::repo::DownloadRepo;
use crate::services::{ClientRoster, JobService, ReleaseService, RequestService, new_id};
use crate::{Error, Result};

pub struct ReleaseSelection {
	pub title: String,
	pub indexer: String,
	pub download_url: String,
	pub info_hash: Option<String>,
	pub protocol: Protocol,
}

impl ReleaseSelection {
	fn fallback_client_ref(&self) -> Option<String> {
		self.info_hash
			.clone()
			.or_else(|| magnet::info_hash(&self.download_url))
	}
}

pub struct GrabService {
	downloads: Arc<dyn DownloadRepo>,
	requests: Arc<RequestService>,
	releases: Arc<ReleaseService>,
	clients: ClientRoster,
	jobs: Arc<JobService>,
}

impl GrabService {
	pub fn new(
		downloads: Arc<dyn DownloadRepo>,
		requests: Arc<RequestService>,
		releases: Arc<ReleaseService>,
		clients: ClientRoster,
		jobs: Arc<JobService>,
	) -> Self {
		Self {
			downloads,
			requests,
			releases,
			clients,
			jobs,
		}
	}

	pub async fn test_download(&self, client_id: &str) -> Result<()> {
		self.clients.by_id(client_id)?.test_connection().await
	}

	pub async fn grab(
		&self,
		request_id: &str,
		selection: Option<ReleaseSelection>,
	) -> Result<Download> {
		if self.requests.get(request_id).await?.is_none() {
			return Err(Error::NotFound(format!("request {request_id}")));
		}

		let active = self
			.downloads
			.find_by_request(request_id)
			.await?
			.filter(|download| download.state != DownloadState::Failed);

		if let Some(active) = active {
			if selection.is_some() {
				return Err(Error::Conflict(reasons::DOWNLOAD_IN_PROGRESS.to_string()));
			}
			return self.finalize(request_id, active).await;
		}

		let selection = match selection {
			Some(selection) => selection,
			None => self.best_release(request_id).await?,
		};

		let client = self.clients.by_protocol(selection.protocol).await?;
		let assigned = client.add(&selection.download_url, GRAB_CATEGORY).await?;

		let now = Utc::now();
		let client_ref = assigned.or_else(|| selection.fallback_client_ref());
		let download = Download {
			id: new_id(),
			request_id: request_id.to_string(),
			client: client.id().to_string(),
			category: GRAB_CATEGORY.to_string(),
			release_title: selection.title,
			indexer: selection.indexer,
			download_url: selection.download_url,
			client_ref,
			state: DownloadState::Queued,
			progress: 0,
			created_at: now,
			updated_at: now,
		};
		self.downloads
			.create(&download)
			.await
			.map_err(|error| match error {
				Error::Conflict(_) => Error::Conflict(reasons::DOWNLOAD_IN_PROGRESS.to_string()),
				other => other,
			})?;

		self.finalize(request_id, download).await
	}

	async fn finalize(&self, request_id: &str, download: Download) -> Result<Download> {
		let now = Utc::now();
		if download.client_ref.is_none() {
			self.downloads
				.update_status(&download.id, DownloadState::Failed, download.progress, now)
				.await?;
			self.requests
				.mark_failed(request_id, Some("download has no trackable reference"))
				.await?;
			return Ok(download);
		}

		self.requests.mark_downloading(request_id).await?;
		if !self
			.jobs
			.has_active(JobType::MonitorDownload, request_id)
			.await?
		{
			let payload = MonitorPayload {
				download_id: download.id.clone(),
				misses: 0,
				stalls: 0,
			};
			let run_after = now + Duration::seconds(MONITOR_POLL_SECS);
			self.jobs
				.enqueue_for_at(JobType::MonitorDownload, &payload, request_id, run_after)
				.await?;
		}
		Ok(download)
	}

	pub async fn for_request(&self, request_id: &str) -> Result<Option<Download>> {
		self.downloads.find_by_request(request_id).await
	}

	pub async fn cancel(&self, request_id: &str) -> Result<()> {
		let download = self
			.downloads
			.find_by_request(request_id)
			.await?
			.ok_or_else(|| Error::NotFound(format!("download for request {request_id}")))?;

		if let Some(client_ref) = download.client_ref.as_deref()
			&& let Ok(client) = self.clients.by_id(&download.client)
			&& !client.protocol().owes_seeding_at(download.progress)
		{
			let _ = client.remove(client_ref, true).await;
		}

		self.downloads
			.update_status(
				&download.id,
				DownloadState::Failed,
				download.progress,
				Utc::now(),
			)
			.await?;
		self.requests
			.mark_failed(request_id, Some("download cancelled by admin"))
			.await?;
		Ok(())
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
			protocol: best.release.protocol,
		})
	}
}
