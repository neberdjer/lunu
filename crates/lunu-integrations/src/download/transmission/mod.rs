use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use lunu_core::consts::reasons;
use lunu_core::models::{DownloadState, DownloadStatus, Protocol};
use lunu_core::services::SettingsService;
use lunu_core::traits::DownloadClient;
use lunu_core::{Error, Result};
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::http::send_write;
use crate::{integration_error, optional_setting, required_setting};

const PROVIDER_ID: &str = lunu_core::consts::settings::TRANSMISSION;
const REQUEST_TIMEOUT_SECS: u64 = 30;
const SESSION_HEADER: &str = "X-Transmission-Session-Id";
const STATUS_FIELDS: &[&str] = &["status", "percentDone", "downloadDir", "name"];

const TR_STOPPED: i64 = 0;
const TR_DOWNLOADING: i64 = 4;
const TR_SEED_WAIT: i64 = 5;
const TR_SEEDING: i64 = 6;

const SETTING_URL: &str = lunu_core::consts::settings::TRANSMISSION_URL;
const SETTING_USERNAME: &str = lunu_core::consts::settings::TRANSMISSION_USERNAME;
const SETTING_PASSWORD: &str = lunu_core::consts::settings::TRANSMISSION_PASSWORD;
const SETTING_DOWNLOAD_DIR: &str = lunu_core::consts::settings::DOWNLOAD_DIR;

#[derive(Deserialize)]
struct RpcResponse {
	result: String,
	#[serde(default)]
	arguments: Value,
}

#[derive(Deserialize)]
struct TorrentsArguments {
	#[serde(default)]
	torrents: Vec<TorrentInfo>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TorrentInfo {
	status: i64,
	percent_done: f64,
	download_dir: String,
	name: String,
}

fn added_hash(arguments: &Value) -> Option<String> {
	arguments
		.get("torrent-added")
		.or_else(|| arguments.get("torrent-duplicate"))
		.and_then(|added| added.get("hashString"))
		.and_then(Value::as_str)
		.map(str::to_string)
}

fn map_torrent(torrent: TorrentInfo) -> DownloadStatus {
	let complete = torrent.percent_done >= 1.0;
	let state = match torrent.status {
		TR_DOWNLOADING => DownloadState::Downloading,
		TR_SEED_WAIT | TR_SEEDING => DownloadState::Completed,
		TR_STOPPED if complete => DownloadState::Completed,
		_ => DownloadState::Queued,
	};
	let content_path = complete.then(|| format!("{}/{}", torrent.download_dir, torrent.name));
	DownloadStatus {
		state,
		progress: torrent.percent_done,
		content_path,
	}
}

fn rpc_endpoint(base_url: &str) -> String {
	let base_url = base_url.trim_end_matches('/');
	if base_url.ends_with("/rpc") {
		base_url.to_string()
	} else {
		format!("{base_url}/transmission/rpc")
	}
}

pub struct TransmissionClient {
	http: reqwest::Client,
	settings: Arc<SettingsService>,
	session_id: Mutex<Option<String>>,
}

impl TransmissionClient {
	pub fn new(settings: Arc<SettingsService>) -> Self {
		let http = crate::http_client_builder(Duration::from_secs(REQUEST_TIMEOUT_SECS))
			.build()
			.expect("reqwest client builds with static configuration");

		Self {
			http,
			settings,
			session_id: Mutex::new(None),
		}
	}

	async fn rpc(&self, method: &str, arguments: Value) -> Result<Value> {
		let endpoint = rpc_endpoint(
			&required_setting(
				&self.settings,
				SETTING_URL,
				reasons::TRANSMISSION_NOT_CONFIGURED,
			)
			.await?,
		);
		let username = optional_setting(&self.settings, SETTING_USERNAME).await?;
		let password = optional_setting(&self.settings, SETTING_PASSWORD).await?;
		let body = json!({ "method": method, "arguments": arguments });

		let mut response = self.send(&endpoint, &body, &username, &password).await?;
		if response.status() == StatusCode::CONFLICT {
			let session = response
				.headers()
				.get(SESSION_HEADER)
				.and_then(|value| value.to_str().ok())
				.map(str::to_string);
			*self.session_id.lock().expect("session lock") = session;
			response = self.send(&endpoint, &body, &username, &password).await?;
		}
		let response = response.error_for_status().map_err(integration_error)?;

		let parsed: RpcResponse = crate::http::bounded_json(response).await?;
		if parsed.result != "success" {
			return Err(Error::Integration(format!(
				"transmission rejected the call: {}",
				parsed.result
			)));
		}
		Ok(parsed.arguments)
	}

	async fn send(
		&self,
		endpoint: &str,
		body: &Value,
		username: &Option<String>,
		password: &Option<String>,
	) -> Result<reqwest::Response> {
		send_write(|| {
			let mut request = self.http.post(endpoint).json(body);
			if let Some(session) = self.session_id.lock().expect("session lock").clone() {
				request = request.header(SESSION_HEADER, session);
			}
			if let Some(username) = username {
				request = request.basic_auth(username, password.as_deref());
			}
			request
		})
		.await
	}
}

#[async_trait]
impl DownloadClient for TransmissionClient {
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
		let mut arguments = json!({ "filename": download_url, "labels": [category] });
		if let Some(dir) = optional_setting(&self.settings, SETTING_DOWNLOAD_DIR).await? {
			arguments["download-dir"] = Value::String(dir);
		}
		let added = self.rpc("torrent-add", arguments).await?;
		Ok(added_hash(&added))
	}

	async fn status(&self, client_ref: &str) -> Result<Option<DownloadStatus>> {
		let arguments = self
			.rpc(
				"torrent-get",
				json!({ "ids": [client_ref], "fields": STATUS_FIELDS }),
			)
			.await?;
		let parsed: TorrentsArguments =
			serde_json::from_value(arguments).map_err(integration_error)?;
		Ok(parsed.torrents.into_iter().next().map(map_torrent))
	}

	async fn remove(&self, client_ref: &str, delete_files: bool) -> Result<()> {
		self.rpc(
			"torrent-remove",
			json!({ "ids": [client_ref], "delete-local-data": delete_files }),
		)
		.await?;
		Ok(())
	}

	async fn test_connection(&self) -> Result<()> {
		self.rpc("session-get", json!({})).await?;
		Ok(())
	}
}

#[cfg(test)]
mod tests;
