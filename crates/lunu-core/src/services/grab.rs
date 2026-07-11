use std::sync::Arc;

use chrono::Utc;

use crate::consts::download::GRAB_CATEGORY;
use crate::consts::reasons;
use crate::models::{Download, DownloadState};
use crate::repo::DownloadRepo;
use crate::services::{ReleaseService, RequestService, new_id};
use crate::traits::DownloadClient;
use crate::{Error, Result};

pub struct ReleaseSelection {
	pub title: String,
	pub indexer: String,
	pub download_url: String,
}

pub struct GrabService {
	downloads: Arc<dyn DownloadRepo>,
	requests: Arc<RequestService>,
	releases: Arc<ReleaseService>,
	client: Arc<dyn DownloadClient>,
}

impl GrabService {
	pub fn new(
		downloads: Arc<dyn DownloadRepo>,
		requests: Arc<RequestService>,
		releases: Arc<ReleaseService>,
		client: Arc<dyn DownloadClient>,
	) -> Self {
		Self {
			downloads,
			requests,
			releases,
			client,
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
		let download = Download {
			id: new_id(),
			request_id: request_id.to_string(),
			client: self.client.id().to_string(),
			category: GRAB_CATEGORY.to_string(),
			release_title: selection.title,
			indexer: selection.indexer,
			download_url: selection.download_url,
			state: DownloadState::Queued,
			created_at: now,
			updated_at: now,
		};
		self.downloads.create(&download).await?;

		self.requests.mark_downloading(request_id).await?;

		Ok(download)
	}

	pub async fn for_request(&self, request_id: &str) -> Result<Option<Download>> {
		self.downloads.find_by_request(request_id).await
	}

	pub async fn list(&self) -> Result<Vec<Download>> {
		self.downloads.list().await
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
		})
	}
}
