use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use lunu_core::consts::reasons;
use lunu_core::models::{DownloadState, DownloadStatus, Protocol};
use lunu_core::services::SettingsService;
use lunu_core::traits::DownloadClient;
use lunu_core::{Error, Result};
use reqwest::header::REFERER;
use reqwest::multipart::Form;
use reqwest::{RequestBuilder, StatusCode};
use serde::Deserialize;

use crate::http::send_with_retry;
use crate::{integration_error, optional_setting, required_setting};

#[derive(Deserialize)]
struct TorrentInfo {
	#[serde(default)]
	hash: String,
	state: String,
	progress: f64,
	content_path: Option<String>,
}

fn auth_failed() -> Error {
	Error::Validation(reasons::QBITTORRENT_AUTH_FAILED.to_string())
}

fn authorize(request: RequestBuilder, api_key: &Option<String>) -> RequestBuilder {
	match api_key {
		Some(key) => request.bearer_auth(key),
		None => request,
	}
}

fn select_torrent(torrents: Vec<TorrentInfo>, info_hash: &str) -> Option<DownloadStatus> {
	torrents
		.into_iter()
		.find(|torrent| torrent.hash.eq_ignore_ascii_case(info_hash))
		.map(|torrent| DownloadStatus {
			state: map_state(&torrent.state, torrent.progress),
			progress: torrent.progress,
			content_path: torrent.content_path,
		})
}

fn map_state(state: &str, progress: f64) -> DownloadState {
	match state {
		"error" | "missingFiles" => DownloadState::Failed,
		"queuedDL" | "pausedDL" | "stoppedDL" | "checkingDL" | "checkingResumeData"
		| "checkingUP" | "allocating" | "moving" => DownloadState::Queued,
		"uploading" | "stalledUP" | "queuedUP" | "forcedUP" | "pausedUP" => {
			DownloadState::Completed
		}
		_ if progress >= 1.0 => DownloadState::Completed,
		_ => DownloadState::Downloading,
	}
}

const PROVIDER_ID: &str = lunu_core::consts::settings::QBITTORRENT;
const REQUEST_TIMEOUT_SECS: u64 = 30;
const OK_BODY: &str = "Ok.";

const SETTING_URL: &str = lunu_core::consts::settings::QBITTORRENT_URL;
const SETTING_API_KEY: &str = lunu_core::consts::settings::QBITTORRENT_API_KEY;
const SETTING_USERNAME: &str = lunu_core::consts::settings::QBITTORRENT_USERNAME;
const SETTING_PASSWORD: &str = lunu_core::consts::settings::QBITTORRENT_PASSWORD;
const SETTING_DOWNLOAD_DIR: &str = lunu_core::consts::settings::DOWNLOAD_DIR;

pub struct QbittorrentClient {
	http: reqwest::Client,
	settings: Arc<SettingsService>,
	logged_in: AtomicBool,
	category_ready: AtomicBool,
}

impl QbittorrentClient {
	pub fn new(settings: Arc<SettingsService>) -> Self {
		let http = crate::http_client_builder(Duration::from_secs(REQUEST_TIMEOUT_SECS))
			.cookie_store(true)
			.build()
			.expect("reqwest client builds with static configuration");

		Self {
			http,
			settings,
			logged_in: AtomicBool::new(false),
			category_ready: AtomicBool::new(false),
		}
	}

	async fn ensure_category(
		&self,
		base_url: &str,
		api_key: &Option<String>,
		category: &str,
	) -> Result<()> {
		if self.category_ready.load(Ordering::Relaxed) {
			return Ok(());
		}

		let mut form = vec![("category", category.to_string())];
		if let Some(dir) = optional_setting(&self.settings, SETTING_DOWNLOAD_DIR).await? {
			form.push(("savePath", dir));
		}

		authorize(
			self.http
				.post(format!("{base_url}/api/v2/torrents/createCategory"))
				.header(REFERER, base_url)
				.form(&form),
			api_key,
		)
		.send()
		.await
		.map_err(integration_error)?;

		self.category_ready.store(true, Ordering::Relaxed);
		Ok(())
	}

	async fn required(&self, key: &str) -> Result<String> {
		required_setting(&self.settings, key, reasons::QBITTORRENT_NOT_CONFIGURED).await
	}

	async fn prepare(&self) -> Result<(String, Option<String>)> {
		let base_url = self.required(SETTING_URL).await?;
		let base_url = base_url.trim_end_matches('/').to_string();
		let api_key = self.authenticate(&base_url).await?;
		Ok((base_url, api_key))
	}

	async fn authenticate(&self, base_url: &str) -> Result<Option<String>> {
		let api_key = optional_setting(&self.settings, SETTING_API_KEY).await?;
		if api_key.is_none() && !self.logged_in.load(Ordering::Relaxed) {
			self.login(base_url).await?;
			self.logged_in.store(true, Ordering::Relaxed);
		}
		Ok(api_key)
	}

	fn check_response(&self, response: reqwest::Response) -> Result<reqwest::Response> {
		if response.status() == StatusCode::FORBIDDEN {
			self.logged_in.store(false, Ordering::Relaxed);
			return Err(auth_failed());
		}
		response.error_for_status().map_err(integration_error)
	}

	async fn login(&self, base_url: &str) -> Result<()> {
		let username = self.required(SETTING_USERNAME).await?;
		let password = self.required(SETTING_PASSWORD).await?;

		let response = self
			.http
			.post(format!("{base_url}/api/v2/auth/login"))
			.header(REFERER, base_url)
			.form(&[("username", username), ("password", password)])
			.send()
			.await
			.map_err(integration_error)?;

		if response.status() == StatusCode::FORBIDDEN {
			return Err(auth_failed());
		}

		let body = response.text().await.map_err(integration_error)?;
		if body.trim() != OK_BODY {
			return Err(auth_failed());
		}

		Ok(())
	}
}

#[async_trait]
impl DownloadClient for QbittorrentClient {
	fn id(&self) -> &'static str {
		PROVIDER_ID
	}

	fn protocol(&self) -> Protocol {
		Protocol::Torrent
	}

	async fn is_configured(&self) -> Result<bool> {
		crate::setting_present(&self.settings, SETTING_URL).await
	}

	async fn add(&self, download_url: &str, category: &str) -> Result<Option<String>> {
		let (base_url, api_key) = self.prepare().await?;
		let _ = self.ensure_category(&base_url, &api_key, category).await;

		let mut form = Form::new()
			.text("urls", download_url.to_string())
			.text("category", category.to_string())
			.text("autoTMM", "false");

		if let Some(dir) = optional_setting(&self.settings, SETTING_DOWNLOAD_DIR).await? {
			form = form.text("savepath", dir);
		}

		let request = self
			.http
			.post(format!("{base_url}/api/v2/torrents/add"))
			.header(REFERER, base_url.as_str())
			.multipart(form);
		let response = self.check_response(
			authorize(request, &api_key)
				.send()
				.await
				.map_err(integration_error)?,
		)?;

		let body = response.text().await.map_err(integration_error)?;
		if body.trim() != OK_BODY {
			return Err(Error::Integration(format!(
				"qbittorrent rejected the torrent: {}",
				body.trim()
			)));
		}

		Ok(None)
	}

	async fn status(&self, client_ref: &str) -> Result<Option<DownloadStatus>> {
		let (base_url, api_key) = self.prepare().await?;
		let hashes = client_ref.to_ascii_lowercase();

		let response = send_with_retry(|| {
			authorize(
				self.http
					.get(format!("{base_url}/api/v2/torrents/info"))
					.header(REFERER, base_url.as_str())
					.query(&[("hashes", hashes.as_str())]),
				&api_key,
			)
		})
		.await?;
		let response = self.check_response(response)?;

		let torrents: Vec<TorrentInfo> = response.json().await.map_err(integration_error)?;
		Ok(select_torrent(torrents, &hashes))
	}

	async fn remove(&self, client_ref: &str, delete_files: bool) -> Result<()> {
		let (base_url, api_key) = self.prepare().await?;
		let hashes = client_ref.to_ascii_lowercase();
		let delete_files = if delete_files { "true" } else { "false" };

		let request = self
			.http
			.post(format!("{base_url}/api/v2/torrents/delete"))
			.header(REFERER, base_url.as_str())
			.form(&[("hashes", hashes.as_str()), ("deleteFiles", delete_files)]);
		let response = authorize(request, &api_key)
			.send()
			.await
			.map_err(integration_error)?;

		self.check_response(response)?;
		Ok(())
	}

	async fn test_connection(&self) -> Result<()> {
		let (base_url, api_key) = self.prepare().await?;

		let response = send_with_retry(|| {
			authorize(
				self.http
					.get(format!("{base_url}/api/v2/app/version"))
					.header(REFERER, base_url.as_str()),
				&api_key,
			)
		})
		.await?;

		self.check_response(response)?;
		Ok(())
	}
}

#[cfg(test)]
mod tests;
