use std::sync::Arc;

use crate::consts::library::{
	SETTING_IMPORT_KEEP_EXTENSIONS, SETTING_IMPORT_UNLISTED, SETTING_LIBRARY_DIR,
	SETTING_WRITE_SIDECAR,
};
use crate::consts::reasons;
use crate::consts::settings::TOGGLE_ON;
use crate::helpers::{naming, opf};
use crate::models::{ImportFilter, Placement, Request};
use crate::repo::DownloadRepo;
use crate::services::{MediaService, MergeService, RequestService, SettingsService};
use crate::traits::{Importer, Sidecar, SidecarWriter};
use crate::{Error, Result};

pub struct ImportService {
	downloads: Arc<dyn DownloadRepo>,
	requests: Arc<RequestService>,
	settings: Arc<SettingsService>,
	importer: Arc<dyn Importer>,
	media: Arc<MediaService>,
	merges: Arc<MergeService>,
	sidecar: Arc<dyn SidecarWriter>,
}

impl ImportService {
	pub fn new(
		downloads: Arc<dyn DownloadRepo>,
		requests: Arc<RequestService>,
		settings: Arc<SettingsService>,
		importer: Arc<dyn Importer>,
		media: Arc<MediaService>,
		merges: Arc<MergeService>,
		sidecar: Arc<dyn SidecarWriter>,
	) -> Self {
		Self {
			downloads,
			requests,
			settings,
			importer,
			media,
			merges,
			sidecar,
		}
	}

	pub async fn import(&self, download_id: &str, content_path: &str) -> Result<()> {
		let Some(download) = self.downloads.find_by_id(download_id).await? else {
			return Ok(());
		};
		let Some(request) = self.requests.get(&download.request_id).await? else {
			return Ok(());
		};

		let settings = self
			.settings
			.resolve_many(&[
				SETTING_LIBRARY_DIR,
				SETTING_IMPORT_KEEP_EXTENSIONS,
				SETTING_IMPORT_UNLISTED,
				SETTING_WRITE_SIDECAR,
			])
			.await?;
		let library = settings
			.get(SETTING_LIBRARY_DIR)
			.ok_or_else(|| Error::Validation(reasons::LIBRARY_NOT_CONFIGURED.to_string()))?;
		let writes_sidecar =
			settings.get(SETTING_WRITE_SIDECAR).map(String::as_str) == Some(TOGGLE_ON);

		let destination = naming::destination(
			library,
			request.author.as_deref(),
			&request.title,
			request.series_name.as_deref(),
			request.series_sequence.as_deref(),
		);
		let unlisted: Placement = settings
			.get(SETTING_IMPORT_UNLISTED)
			.map(|value| value.parse())
			.transpose()?
			.unwrap_or_default();
		let filter = ImportFilter::new(
			settings
				.get(SETTING_IMPORT_KEEP_EXTENSIONS)
				.map(String::as_str)
				.unwrap_or_default(),
			unlisted,
			writes_sidecar,
		);
		self.importer
			.import(content_path, &destination, &filter)
			.await?;
		if writes_sidecar {
			self.write_sidecar(&request, &destination).await?;
		}
		let media_id = self.media.record(&request, &destination).await?;
		self.requests.mark_available(&download.request_id).await?;

		self.merges.try_request(&media_id).await;
		Ok(())
	}

	async fn write_sidecar(&self, request: &Request, destination: &str) -> Result<()> {
		let opf = opf::metadata_opf(&opf::OpfBook {
			title: &request.title,
			author: request.author.as_deref(),
			series_name: request.series_name.as_deref(),
			series_sequence: request.series_sequence.as_deref(),
			asin: request.asin.as_deref(),
		});
		self.sidecar
			.write(&Sidecar {
				directory: destination,
				opf: &opf,
				cover_url: request.cover_url.as_deref(),
			})
			.await
	}
}
