use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use lunu_core::consts::reasons;
use lunu_core::models::{DownloadState, DownloadStatus, Protocol};
use lunu_core::services::SettingsService;
use lunu_core::traits::DownloadClient;
use lunu_core::{Error, Result};
use serde::Deserialize;

use crate::http::{get_json, send_write};
use crate::{integration_error, required_setting};

const PROVIDER_ID: &str = lunu_core::consts::settings::SABNZBD;
const REQUEST_TIMEOUT_SECS: u64 = 30;

const SETTING_URL: &str = lunu_core::consts::settings::SABNZBD_URL;
const SETTING_API_KEY: &str = lunu_core::consts::settings::SABNZBD_API_KEY;

#[derive(Deserialize)]
struct AddResponse {
	status: bool,
	#[serde(default)]
	nzo_ids: Vec<String>,
	error: Option<String>,
}

#[derive(Deserialize)]
struct QueueResponse {
	queue: Option<QueueSlots>,
	error: Option<String>,
}

#[derive(Deserialize)]
struct QueueSlots {
	#[serde(default)]
	slots: Vec<QueueSlot>,
}

#[derive(Deserialize)]
struct HistoryResponse {
	history: Option<HistorySlots>,
	error: Option<String>,
}

#[derive(Deserialize)]
struct HistorySlots {
	#[serde(default)]
	slots: Vec<HistorySlot>,
}

#[derive(Deserialize)]
struct QueueSlot {
	nzo_id: String,
	status: String,
	percentage: String,
}

#[derive(Deserialize)]
struct HistorySlot {
	nzo_id: String,
	status: String,
	storage: Option<String>,
}

fn queue_status(slot: &QueueSlot) -> DownloadStatus {
	let progress = slot.percentage.parse::<f64>().unwrap_or(0.0) / 100.0;
	let state = match slot.status.as_str() {
		"Queued" | "Paused" | "Grabbing" | "Fetching" | "Propagating" | "Checking" => {
			DownloadState::Queued
		}
		_ => DownloadState::Downloading,
	};
	DownloadStatus {
		state,
		progress,
		content_path: None,
	}
}

fn history_status(slot: HistorySlot) -> DownloadStatus {
	match slot.status.as_str() {
		"Completed" => DownloadStatus {
			state: DownloadState::Completed,
			progress: 1.0,
			content_path: slot.storage,
		},
		"Failed" => DownloadStatus {
			state: DownloadState::Failed,
			progress: 0.0,
			content_path: None,
		},
		_ => DownloadStatus {
			state: DownloadState::Downloading,
			progress: 1.0,
			content_path: None,
		},
	}
}

fn rejected(error: Option<String>) -> Error {
	Error::Integration(format!(
		"sabnzbd rejected the call: {}",
		error.unwrap_or_else(|| "unknown error".to_string())
	))
}

pub struct SabnzbdClient {
	http: reqwest::Client,
	settings: Arc<SettingsService>,
}

impl SabnzbdClient {
	pub fn new(settings: Arc<SettingsService>) -> Self {
		let http = crate::http_client_builder(Duration::from_secs(REQUEST_TIMEOUT_SECS))
			.build()
			.expect("reqwest client builds with static configuration");

		Self { http, settings }
	}

	async fn prepare(&self) -> Result<(String, String)> {
		let base_url =
			required_setting(&self.settings, SETTING_URL, reasons::SABNZBD_NOT_CONFIGURED).await?;
		let api_key = required_setting(
			&self.settings,
			SETTING_API_KEY,
			reasons::SABNZBD_NOT_CONFIGURED,
		)
		.await?;
		Ok((base_url.trim_end_matches('/').to_string(), api_key))
	}

	async fn call_json<T: serde::de::DeserializeOwned>(
		&self,
		params: &[(&str, &str)],
	) -> Result<T> {
		let (base_url, api_key) = self.prepare().await?;
		self.call_json_with(&base_url, &api_key, params).await
	}

	async fn call_json_with<T: serde::de::DeserializeOwned>(
		&self,
		base_url: &str,
		api_key: &str,
		params: &[(&str, &str)],
	) -> Result<T> {
		get_json(|| {
			self.http
				.get(format!("{base_url}/api"))
				.query(params)
				.query(&[("apikey", api_key), ("output", "json")])
		})
		.await
	}

	async fn call_with(
		&self,
		base_url: &str,
		api_key: &str,
		params: &[(&str, &str)],
	) -> Result<()> {
		let response = send_write(|| {
			self.http
				.get(format!("{base_url}/api"))
				.query(params)
				.query(&[("apikey", api_key), ("output", "json")])
		})
		.await?;
		response.error_for_status().map_err(integration_error)?;
		Ok(())
	}
}

#[async_trait]
impl DownloadClient for SabnzbdClient {
	fn id(&self) -> &'static str {
		PROVIDER_ID
	}

	fn protocol(&self) -> Protocol {
		Protocol::Usenet
	}

	async fn is_configured(&self) -> Result<bool> {
		crate::setting_present(&self.settings, SETTING_URL).await
	}

	async fn add(&self, download_url: &str, category: &str) -> Result<Option<String>> {
		let added: AddResponse = self
			.call_json(&[
				("mode", "addurl"),
				("name", download_url),
				("cat", category),
			])
			.await?;
		if !added.status {
			return Err(rejected(added.error));
		}
		Ok(added.nzo_ids.into_iter().next())
	}

	async fn status(&self, client_ref: &str) -> Result<Option<DownloadStatus>> {
		let (base_url, api_key) = self.prepare().await?;

		let queue: QueueResponse = self
			.call_json_with(
				&base_url,
				&api_key,
				&[("mode", "queue"), ("nzo_ids", client_ref)],
			)
			.await?;
		let Some(list) = queue.queue else {
			return Err(rejected(queue.error));
		};
		if let Some(slot) = list.slots.iter().find(|slot| slot.nzo_id == client_ref) {
			return Ok(Some(queue_status(slot)));
		}

		let history: HistoryResponse = self
			.call_json_with(
				&base_url,
				&api_key,
				&[("mode", "history"), ("nzo_ids", client_ref)],
			)
			.await?;
		let Some(list) = history.history else {
			return Err(rejected(history.error));
		};
		Ok(list
			.slots
			.into_iter()
			.find(|slot| slot.nzo_id == client_ref)
			.map(history_status))
	}

	async fn remove(&self, client_ref: &str, delete_files: bool) -> Result<()> {
		let (base_url, api_key) = self.prepare().await?;
		let del_files = if delete_files { "1" } else { "0" };
		self.call_with(
			&base_url,
			&api_key,
			&[
				("mode", "queue"),
				("name", "delete"),
				("value", client_ref),
				("del_files", del_files),
			],
		)
		.await?;
		self.call_with(
			&base_url,
			&api_key,
			&[
				("mode", "history"),
				("name", "delete"),
				("value", client_ref),
				("del_files", del_files),
				("archive", "0"),
			],
		)
		.await?;
		Ok(())
	}

	async fn test_connection(&self) -> Result<()> {
		let queue: QueueResponse = self.call_json(&[("mode", "queue"), ("limit", "1")]).await?;
		if queue.queue.is_none() {
			return Err(rejected(queue.error));
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests;
