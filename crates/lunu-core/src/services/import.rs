use std::sync::Arc;

use crate::consts::library::SETTING_LIBRARY_DIR;
use crate::consts::reasons;
use crate::helpers::naming;
use crate::repo::DownloadRepo;
use crate::services::{MediaService, RequestService, SettingsService, nonempty};
use crate::traits::Importer;
use crate::{Error, Result};

pub struct ImportService {
	downloads: Arc<dyn DownloadRepo>,
	requests: Arc<RequestService>,
	settings: Arc<SettingsService>,
	importer: Arc<dyn Importer>,
	media: Arc<MediaService>,
}

impl ImportService {
	pub fn new(
		downloads: Arc<dyn DownloadRepo>,
		requests: Arc<RequestService>,
		settings: Arc<SettingsService>,
		importer: Arc<dyn Importer>,
		media: Arc<MediaService>,
	) -> Self {
		Self {
			downloads,
			requests,
			settings,
			importer,
			media,
		}
	}

	pub async fn import(&self, download_id: &str, content_path: &str) -> Result<()> {
		let Some(download) = self.downloads.find_by_id(download_id).await? else {
			return Ok(());
		};
		let Some(request) = self.requests.get(&download.request_id).await? else {
			return Ok(());
		};

		let library = nonempty(self.settings.get(SETTING_LIBRARY_DIR).await?)
			.ok_or_else(|| Error::Validation(reasons::LIBRARY_NOT_CONFIGURED.to_string()))?;

		let destination = naming::destination(&library, request.author.as_deref(), &request.title);
		self.importer.import(content_path, &destination).await?;
		self.media.record(&request, &destination).await?;
		self.requests.mark_available(&download.request_id).await?;
		Ok(())
	}
}
