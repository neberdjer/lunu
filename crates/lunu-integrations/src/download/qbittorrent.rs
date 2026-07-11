use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use lunu_core::consts::reasons;
use lunu_core::services::SettingsService;
use lunu_core::traits::DownloadClient;
use lunu_core::{Error, Result};
use reqwest::StatusCode;
use reqwest::header::REFERER;
use reqwest::multipart::Form;

use crate::{integration_error, optional_setting, required_setting};

const PROVIDER_ID: &str = "qbittorrent";
const REQUEST_TIMEOUT_SECS: u64 = 30;
const OK_BODY: &str = "Ok.";

const SETTING_URL: &str = "qbittorrent_url";
const SETTING_API_KEY: &str = "qbittorrent_api_key";
const SETTING_USERNAME: &str = "qbittorrent_username";
const SETTING_PASSWORD: &str = "qbittorrent_password";
const SETTING_DOWNLOAD_DIR: &str = "download_dir";

pub struct QbittorrentClient {
	http: reqwest::Client,
	settings: Arc<SettingsService>,
}

impl QbittorrentClient {
	pub fn new(settings: Arc<SettingsService>) -> Self {
		let http = reqwest::Client::builder()
			.user_agent(concat!("lunu/", env!("CARGO_PKG_VERSION")))
			.timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
			.cookie_store(true)
			.build()
			.expect("reqwest client builds with static configuration");

		Self { http, settings }
	}

	async fn required(&self, key: &str) -> Result<String> {
		required_setting(&self.settings, key, reasons::QBITTORRENT_NOT_CONFIGURED).await
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
			return Err(Error::Validation(
				reasons::QBITTORRENT_AUTH_FAILED.to_string(),
			));
		}

		let body = response.text().await.map_err(integration_error)?;
		if body.trim() != OK_BODY {
			return Err(Error::Validation(
				reasons::QBITTORRENT_AUTH_FAILED.to_string(),
			));
		}

		Ok(())
	}
}

#[async_trait]
impl DownloadClient for QbittorrentClient {
	fn id(&self) -> &'static str {
		PROVIDER_ID
	}

	async fn add(&self, download_url: &str, category: &str) -> Result<()> {
		let base_url = self.required(SETTING_URL).await?;
		let base_url = base_url.trim_end_matches('/');

		let api_key = optional_setting(&self.settings, SETTING_API_KEY).await?;
		if api_key.is_none() {
			self.login(base_url).await?;
		}

		let mut form = Form::new()
			.text("urls", download_url.to_string())
			.text("category", category.to_string())
			.text("autoTMM", "false");

		if let Some(dir) = optional_setting(&self.settings, SETTING_DOWNLOAD_DIR).await? {
			form = form.text("savepath", dir);
		}

		let mut request = self
			.http
			.post(format!("{base_url}/api/v2/torrents/add"))
			.header(REFERER, base_url)
			.multipart(form);
		if let Some(key) = &api_key {
			request = request.bearer_auth(key);
		}

		let response = request.send().await.map_err(integration_error)?;
		if response.status() == StatusCode::FORBIDDEN {
			return Err(Error::Validation(
				reasons::QBITTORRENT_AUTH_FAILED.to_string(),
			));
		}
		let response = response.error_for_status().map_err(integration_error)?;

		let body = response.text().await.map_err(integration_error)?;
		if body.trim() != OK_BODY {
			return Err(Error::Integration(format!(
				"qbittorrent rejected the torrent: {}",
				body.trim()
			)));
		}

		Ok(())
	}
}
