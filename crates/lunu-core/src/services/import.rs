use std::sync::Arc;

use crate::consts::library::SETTING_LIBRARY_DIR;
use crate::consts::reasons;
use crate::helpers::naming;
use crate::repo::DownloadRepo;
use crate::services::{RequestService, SettingsService};
use crate::traits::Importer;
use crate::{Error, Result};

pub struct ImportService {
	downloads: Arc<dyn DownloadRepo>,
	requests: Arc<RequestService>,
	settings: Arc<SettingsService>,
	importer: Arc<dyn Importer>,
}

impl ImportService {
	pub fn new(
		downloads: Arc<dyn DownloadRepo>,
		requests: Arc<RequestService>,
		settings: Arc<SettingsService>,
		importer: Arc<dyn Importer>,
	) -> Self {
		Self {
			downloads,
			requests,
			settings,
			importer,
		}
	}

	pub async fn import(&self, download_id: &str, content_path: &str) -> Result<()> {
		let Some(download) = self.downloads.find_by_id(download_id).await? else {
			return Ok(());
		};
		let Some(request) = self.requests.get(&download.request_id).await? else {
			return Ok(());
		};

		let library = self
			.settings
			.get(SETTING_LIBRARY_DIR)
			.await?
			.map(|value| value.trim().to_string())
			.filter(|value| !value.is_empty())
			.ok_or_else(|| Error::Validation(reasons::LIBRARY_NOT_CONFIGURED.to_string()))?;

		let destination = naming::destination(&library, request.author.as_deref(), &request.title);
		self.importer.import(content_path, &destination).await?;
		self.requests.mark_available(&download.request_id).await?;
		Ok(())
	}
}
