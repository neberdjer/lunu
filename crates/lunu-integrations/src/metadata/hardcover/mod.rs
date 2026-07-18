use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::consts::metadata::METADATA_HARDCOVER_API_KEY;
use lunu_core::models::{Book, Chapters, ExternalId, IdScheme, SeriesSummary};
use lunu_core::services::SettingsService;
use lunu_core::traits::MetadataProvider;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

mod edition;
mod search;

use crate::{integration_error, optional_setting};

const PROVIDER_ID: &str = lunu_core::consts::metadata::HARDCOVER_PROVIDER;
const REQUEST_TIMEOUT_SECS: u64 = 15;
const ACCEPTS: &[IdScheme] = &[IdScheme::Isbn];
const BASE: &str = "https://api.hardcover.app/v1/graphql";
const SEARCH_LIMIT: i64 = 10;
pub(super) const NARRATOR_ROLE: &str = "Narrator";
pub(super) use lunu_core::consts::metadata::METADATA_GENRE_LIMIT as GENRE_LIMIT;

pub(super) fn format_rating(rating: Option<f64>) -> Option<String> {
	rating
		.filter(|rating| *rating > 0.0)
		.map(|rating| format!("{rating:.2}"))
}

const EDITION_QUERY: &str = "query($isbn: String!) { \
	editions(where: {_or: [{isbn_13: {_eq: $isbn}}, {isbn_10: {_eq: $isbn}}]}, limit: 1) { \
	isbn_13 isbn_10 asin audio_seconds release_date \
	language { language } publisher { name } \
	contributions { contribution author { name } } \
	book { title subtitle description release_date rating cached_tags(path: \"Genre\") image { url } } } }";

const SEARCH_QUERY: &str = "query($q: String!, $per: Int!) { \
	search(query: $q, query_type: \"Book\", per_page: $per, page: 1) { results } }";

pub struct HardcoverProvider {
	client: reqwest::Client,
	settings: Arc<SettingsService>,
}

impl HardcoverProvider {
	pub fn new(settings: Arc<SettingsService>) -> Self {
		let client = crate::http_client_builder(Duration::from_secs(REQUEST_TIMEOUT_SECS))
			.build()
			.expect("reqwest client builds with static configuration");

		Self { client, settings }
	}

	async fn graphql<T: DeserializeOwned>(&self, token: &str, body: &Value) -> Result<T> {
		let response: GraphQlResponse<T> =
			crate::http::get_json(|| self.client.post(BASE).bearer_auth(token).json(body)).await?;

		if let Some(error) = response.errors.into_iter().next() {
			return Err(integration_error(error.message));
		}
		response
			.data
			.ok_or_else(|| integration_error("hardcover returned no data"))
	}

	async fn token(&self) -> Result<Option<String>> {
		Ok(optional_setting(&self.settings, METADATA_HARDCOVER_API_KEY)
			.await?
			.map(|token| normalize_token(&token)))
	}
}

#[async_trait]
impl MetadataProvider for HardcoverProvider {
	fn id(&self) -> &'static str {
		PROVIDER_ID
	}

	fn accepts(&self) -> &[IdScheme] {
		ACCEPTS
	}

	async fn search(&self, query: &str, _region: &str, _page: i64) -> Result<Vec<Book>> {
		let Some(token) = self.token().await? else {
			return Ok(Vec::new());
		};
		let body = json!({"query": SEARCH_QUERY, "variables": {"q": query, "per": SEARCH_LIMIT}});
		let data: search::SearchData = self.graphql(&token, &body).await?;
		Ok(data.into_books())
	}

	async fn get_book(&self, id: &ExternalId, _region: &str) -> Result<Option<Book>> {
		let Some(isbn) = id.value_for(IdScheme::Isbn) else {
			return Ok(None);
		};
		let Some(token) = self.token().await? else {
			return Ok(None);
		};
		let body = json!({"query": EDITION_QUERY, "variables": {"isbn": isbn}});
		let data: edition::EditionsData = self.graphql(&token, &body).await?;
		Ok(data.into_book())
	}

	async fn get_chapters(&self, _id: &ExternalId, _region: &str) -> Result<Option<Chapters>> {
		Ok(None)
	}

	async fn similar(&self, _id: &ExternalId, _region: &str) -> Result<Vec<Book>> {
		Ok(Vec::new())
	}

	async fn books_by_author(&self, _author: &ExternalId, _region: &str) -> Result<Vec<Book>> {
		Ok(Vec::new())
	}

	async fn search_series(&self, _query: &str, _region: &str) -> Result<Vec<SeriesSummary>> {
		Ok(Vec::new())
	}

	async fn series_books(
		&self,
		_name: &str,
		_id: Option<&ExternalId>,
		_region: &str,
	) -> Result<Vec<Book>> {
		Ok(Vec::new())
	}
}

fn normalize_token(token: &str) -> String {
	token
		.strip_prefix("Bearer ")
		.or_else(|| token.strip_prefix("bearer "))
		.unwrap_or(token)
		.trim()
		.to_string()
}

#[derive(Deserialize)]
struct GraphQlResponse<T> {
	data: Option<T>,
	#[serde(default)]
	errors: Vec<GraphQlError>,
}

#[derive(Deserialize)]
struct GraphQlError {
	message: String,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_pasted_token_is_reduced_to_its_bare_value_however_it_was_copied() {
		assert_eq!(normalize_token("eyJ.abc"), "eyJ.abc");
		assert_eq!(normalize_token("Bearer eyJ.abc"), "eyJ.abc");
		assert_eq!(normalize_token("bearer eyJ.abc"), "eyJ.abc");
		assert_eq!(normalize_token("  eyJ.abc  "), "eyJ.abc");
	}
}
