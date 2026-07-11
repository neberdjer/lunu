use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use lunu_core::consts::reasons;
use lunu_core::models::{Protocol, Release};
use lunu_core::services::SettingsService;
use lunu_core::traits::Indexer;
use lunu_core::{Error, Result};
use reqwest::StatusCode;
use serde::Deserialize;

use crate::http::send_with_retry;
use crate::{integration_error, required_setting};

const PROVIDER_ID: &str = "prowlarr";
const AUDIOBOOK_CATEGORY: &str = "3030";
const ALL_INDEXERS: &str = "-1";
const SEARCH_TYPE: &str = "search";
const REQUEST_TIMEOUT_SECS: u64 = 60;
const SETTING_URL: &str = lunu_core::consts::settings::PROWLARR_URL;
const SETTING_API_KEY: &str = lunu_core::consts::settings::PROWLARR_API_KEY;

pub struct ProwlarrClient {
	http: reqwest::Client,
	settings: Arc<SettingsService>,
}

impl ProwlarrClient {
	pub fn new(settings: Arc<SettingsService>) -> Self {
		let http = reqwest::Client::builder()
			.user_agent(concat!("lunu/", env!("CARGO_PKG_VERSION")))
			.timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
			.build()
			.expect("reqwest client builds with static configuration");

		Self { http, settings }
	}

	async fn setting(&self, key: &str) -> Result<String> {
		required_setting(&self.settings, key, reasons::PROWLARR_NOT_CONFIGURED).await
	}
}

#[async_trait]
impl Indexer for ProwlarrClient {
	fn id(&self) -> &'static str {
		PROVIDER_ID
	}

	async fn search(&self, query: &str) -> Result<Vec<Release>> {
		let base_url = self.setting(SETTING_URL).await?;
		let api_key = self.setting(SETTING_API_KEY).await?;

		let url = format!("{}/api/v1/search", base_url.trim_end_matches('/'));
		let response = send_with_retry(|| {
			self.http
				.get(&url)
				.header("X-Api-Key", api_key.as_str())
				.query(&[
					("query", query),
					("indexerIds", ALL_INDEXERS),
					("categories", AUDIOBOOK_CATEGORY),
					("type", SEARCH_TYPE),
				])
		})
		.await?;

		if response.status() == StatusCode::UNAUTHORIZED {
			return Err(Error::Validation(
				reasons::PROWLARR_UNAUTHORIZED.to_string(),
			));
		}

		let response = response.error_for_status().map_err(integration_error)?;
		let results: Vec<ProwlarrRelease> = response.json().await.map_err(integration_error)?;
		Ok(results
			.into_iter()
			.filter_map(ProwlarrRelease::into_release)
			.collect())
	}
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProwlarrRelease {
	title: String,
	indexer: Option<String>,
	size: Option<i64>,
	seeders: Option<i64>,
	leechers: Option<i64>,
	protocol: Option<String>,
	download_url: Option<String>,
	info_hash: Option<String>,
	info_url: Option<String>,
	publish_date: Option<String>,
}

impl ProwlarrRelease {
	fn into_release(self) -> Option<Release> {
		if self.protocol.as_deref() != Some(Protocol::Torrent.as_str()) {
			return None;
		}

		Some(Release {
			title: self.title,
			indexer: self.indexer.unwrap_or_default(),
			protocol: Protocol::Torrent,
			size: self.size.unwrap_or(0),
			seeders: self.seeders.unwrap_or(0),
			leechers: self.leechers.unwrap_or(0),
			download_url: self.download_url?,
			info_hash: self.info_hash,
			info_url: self.info_url,
			publish_date: self.publish_date,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const PROWLARR_SEARCH: &str = r#"[
		{
			"title": "Author - The Hobbit [M4B]",
			"indexer": "MyTracker",
			"size": 524288000,
			"seeders": 42,
			"leechers": 3,
			"protocol": "torrent",
			"downloadUrl": "magnet:?xt=urn:btih:abc",
			"infoUrl": "https://tracker/details",
			"publishDate": "2020-01-01T00:00:00Z"
		},
		{
			"title": "Some Usenet Release",
			"indexer": "NZBTracker",
			"size": 1,
			"protocol": "usenet",
			"downloadUrl": "https://usenet/nzb"
		},
		{
			"title": "Torrent Without Download Url",
			"indexer": "BrokenTracker",
			"protocol": "torrent"
		}
	]"#;

	#[test]
	fn parses_and_filters_torrent_only() {
		let results: Vec<ProwlarrRelease> = serde_json::from_str(PROWLARR_SEARCH).unwrap();
		let releases: Vec<Release> = results
			.into_iter()
			.filter_map(ProwlarrRelease::into_release)
			.collect();

		assert_eq!(releases.len(), 1);
		assert_eq!(releases[0].title, "Author - The Hobbit [M4B]");
		assert_eq!(releases[0].protocol, Protocol::Torrent);
		assert_eq!(releases[0].seeders, 42);
		assert_eq!(releases[0].download_url, "magnet:?xt=urn:btih:abc");
	}
}
