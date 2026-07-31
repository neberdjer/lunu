use std::path::{Path, PathBuf};
use std::time::Duration;

use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::consts::library::{COVER_FILE, METADATA_OPF_FILE};
use lunu_core::traits::{Sidecar, SidecarWriter};

use crate::guard::{
	MAX_FETCH_BYTES, bounded_bytes, guarded_redirect, public_only_dns, url_is_allowed,
};
use crate::http::send_with_retry;
use crate::integration_error;

const REQUEST_TIMEOUT_SECS: u64 = 20;
const STAGING_SUFFIX: &str = "lunu-part";

pub struct FileSidecarWriter {
	client: reqwest::Client,
}

impl Default for FileSidecarWriter {
	fn default() -> Self {
		Self::new()
	}
}

impl FileSidecarWriter {
	pub fn new() -> Self {
		Self {
			// Cover URLs come from community-editable metadata, so the fetch is guarded against
			// SSRF (blocks internal/reserved addresses, even across redirects) and capped in size.
			client: crate::http_client_builder(Duration::from_secs(REQUEST_TIMEOUT_SECS))
				.redirect(guarded_redirect(3))
				.dns_resolver(public_only_dns())
				.build()
				.expect("reqwest client builds with static configuration"),
		}
	}

	async fn cover(&self, url: &str) -> Result<Vec<u8>> {
		let parsed =
			reqwest::Url::parse(url).map_err(|_| integration_error("invalid cover url"))?;
		if !url_is_allowed(&parsed) {
			return Err(integration_error("cover url points at a blocked address"));
		}
		let response = send_with_retry(|| self.client.get(url))
			.await?
			.error_for_status()
			.map_err(integration_error)?;
		bounded_bytes(response, MAX_FETCH_BYTES).await
	}
}

#[async_trait]
impl SidecarWriter for FileSidecarWriter {
	async fn write(&self, sidecar: &Sidecar<'_>) -> Result<()> {
		let dir = PathBuf::from(sidecar.directory);
		let cover = dir.join(COVER_FILE);

		let wanted = sidecar.cover_url.filter(|url| !url.trim().is_empty());
		let image = match wanted {
			Some(url) if !exists(&cover).await => Some(self.cover(url).await?),
			_ => None,
		};

		let opf = sidecar.opf.to_string();
		tokio::task::spawn_blocking(move || {
			std::fs::create_dir_all(&dir).map_err(integration_error)?;
			replace(&dir.join(METADATA_OPF_FILE), opf.as_bytes())?;
			match image {
				Some(image) => replace(&cover, &image),
				None => Ok(()),
			}
		})
		.await
		.map_err(integration_error)?
	}
}

async fn exists(path: &Path) -> bool {
	tokio::fs::try_exists(path).await.unwrap_or(false)
}

fn replace(target: &Path, body: &[u8]) -> Result<()> {
	use std::io::Write;
	let staging = target.with_extension(STAGING_SUFFIX);
	let mut file = match std::fs::OpenOptions::new()
		.write(true)
		.create_new(true)
		.open(&staging)
	{
		Ok(file) => file,
		Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
			std::fs::remove_file(&staging).map_err(integration_error)?;
			std::fs::OpenOptions::new()
				.write(true)
				.create_new(true)
				.open(&staging)
				.map_err(integration_error)?
		}
		Err(error) => return Err(integration_error(error)),
	};
	file.write_all(body).map_err(integration_error)?;
	std::fs::rename(&staging, target).map_err(integration_error)
}
