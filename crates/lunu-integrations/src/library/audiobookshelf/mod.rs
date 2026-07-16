use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use lunu_core::consts::reasons;
use lunu_core::consts::settings::{ABS_API_TOKEN, ABS_LIBRARY_ID, ABS_URL};
use lunu_core::models::LibraryItem;
use lunu_core::services::SettingsService;
use lunu_core::traits::LibrarySource;
use lunu_core::{Error, Result};
use reqwest::StatusCode;
use serde::Deserialize;

fn check_response(response: reqwest::Response) -> Result<reqwest::Response> {
	if matches!(
		response.status(),
		StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
	) {
		return Err(Error::Validation(reasons::ABS_UNAUTHORIZED.to_string()));
	}
	response.error_for_status().map_err(integration_error)
}

use crate::http::send_with_retry;
use crate::{http_client_builder, integration_error, nonempty, optional_setting};

const REQUEST_TIMEOUT_SECS: u64 = 60;
const PAGE_SIZE: i64 = 100;
const MAX_PAGES: i64 = 10_000;

pub struct AbsLibrary {
	http: reqwest::Client,
	settings: Arc<SettingsService>,
}

impl AbsLibrary {
	pub fn new(settings: Arc<SettingsService>) -> Self {
		let http = http_client_builder(Duration::from_secs(REQUEST_TIMEOUT_SECS))
			.build()
			.expect("reqwest client builds with static configuration");
		Self { http, settings }
	}

	async fn config(&self) -> Result<(String, String)> {
		let base = optional_setting(&self.settings, ABS_URL).await?;
		let token = optional_setting(&self.settings, ABS_API_TOKEN).await?;
		match (base, token) {
			(Some(base), Some(token)) => Ok((base.trim_end_matches('/').to_string(), token)),
			_ => Err(Error::Validation(reasons::ABS_NOT_CONFIGURED.to_string())),
		}
	}

	async fn library_ids(&self, base: &str, token: &str) -> Result<Vec<String>> {
		if let Some(id) = optional_setting(&self.settings, ABS_LIBRARY_ID).await? {
			return Ok(vec![id]);
		}

		let url = format!("{base}/api/libraries");
		let response =
			check_response(send_with_retry(|| self.http.get(&url).bearer_auth(token)).await?)?;
		let body: LibrariesResponse = response.json().await.map_err(integration_error)?;
		Ok(body
			.libraries
			.into_iter()
			.filter(|library| library.media_type == "book")
			.map(|library| library.id)
			.collect())
	}

	async fn library_items(
		&self,
		base: &str,
		token: &str,
		library_id: &str,
	) -> Result<Vec<LibraryItem>> {
		let url = format!("{base}/api/libraries/{library_id}/items");
		let mut items = Vec::new();

		for page in 0..MAX_PAGES {
			let response = send_with_retry(|| {
				self.http
					.get(&url)
					.bearer_auth(token)
					.query(&[("limit", PAGE_SIZE.to_string()), ("page", page.to_string())])
			})
			.await?;
			let response = check_response(response)?;
			let body: ItemsResponse = response.json().await.map_err(integration_error)?;

			let count = body.results.len();
			for result in body.results {
				items.push(into_item(base, result));
			}
			if count < PAGE_SIZE as usize {
				return Ok(items);
			}
		}

		tracing::warn!(
			max_pages = MAX_PAGES,
			"audiobookshelf library sync hit the page ceiling; results may be truncated"
		);
		Ok(items)
	}
}

#[async_trait]
impl LibrarySource for AbsLibrary {
	async fn list_items(&self) -> Result<Vec<LibraryItem>> {
		let (base, token) = self.config().await?;
		let mut items = Vec::new();
		for library_id in self.library_ids(&base, &token).await? {
			items.extend(self.library_items(&base, &token, &library_id).await?);
		}
		Ok(items)
	}
}

fn into_item(base: &str, item: AbsItem) -> LibraryItem {
	let metadata = item.media.and_then(|media| media.metadata);
	let cover_url = Some(format!("{base}/api/items/{}/cover", item.id));

	let (title, author, asin, series_raw) = match metadata {
		Some(metadata) => (
			nonempty(metadata.title),
			nonempty(metadata.author_name),
			nonempty(metadata.asin),
			metadata.series_name,
		),
		None => (None, None, None, None),
	};

	let (series_name, series_sequence) = parse_series_name(series_raw);

	LibraryItem {
		abs_item_id: item.id,
		asin,
		title: title.unwrap_or_else(|| "Untitled".to_string()),
		author,
		cover_url,
		series_name,
		series_sequence,
		library_path: item.path.or(item.rel_path).unwrap_or_default(),
	}
}

fn parse_series_name(series_name: Option<String>) -> (Option<String>, Option<String>) {
	let Some(first) = series_name
		.as_deref()
		.and_then(|value| value.split(", ").next())
		.map(str::trim)
		.filter(|value| !value.is_empty())
	else {
		return (None, None);
	};

	if let Some((name, sequence)) = first.rsplit_once(" #") {
		let sequence = sequence.trim();
		let numeric = !sequence.is_empty()
			&& sequence
				.chars()
				.all(|c| c.is_ascii_digit() || c == '.' || c == '-');
		if numeric {
			return (nonempty(Some(name.to_string())), Some(sequence.to_string()));
		}
	}
	(Some(first.to_string()), None)
}

#[derive(Deserialize)]
struct LibrariesResponse {
	#[serde(default)]
	libraries: Vec<AbsLibraryInfo>,
}

#[derive(Deserialize)]
struct AbsLibraryInfo {
	id: String,
	#[serde(default, rename = "mediaType")]
	media_type: String,
}

#[derive(Deserialize)]
struct ItemsResponse {
	#[serde(default, deserialize_with = "lenient_items")]
	results: Vec<AbsItem>,
}

fn lenient_items<'de, D>(deserializer: D) -> std::result::Result<Vec<AbsItem>, D::Error>
where
	D: serde::Deserializer<'de>,
{
	let values = Vec::<serde_json::Value>::deserialize(deserializer)?;
	Ok(values
		.into_iter()
		.filter_map(|value| serde_json::from_value(value).ok())
		.collect())
}

#[derive(Deserialize)]
struct AbsItem {
	id: String,
	#[serde(default)]
	path: Option<String>,
	#[serde(default, rename = "relPath")]
	rel_path: Option<String>,
	#[serde(default)]
	media: Option<AbsMedia>,
}

#[derive(Deserialize)]
struct AbsMedia {
	#[serde(default)]
	metadata: Option<AbsMetadata>,
}

#[derive(Deserialize)]
struct AbsMetadata {
	#[serde(default)]
	title: Option<String>,
	#[serde(default, rename = "authorName")]
	author_name: Option<String>,
	#[serde(default)]
	asin: Option<String>,
	#[serde(default, rename = "seriesName")]
	series_name: Option<String>,
}

#[cfg(test)]
mod tests;
