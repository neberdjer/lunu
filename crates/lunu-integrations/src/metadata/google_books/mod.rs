use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::consts::metadata::METADATA_GOOGLE_BOOKS_API_KEY;
use lunu_core::models::{Book, Chapters, ExternalId, IdScheme, SeriesSummary};
use lunu_core::services::SettingsService;
use lunu_core::traits::MetadataProvider;

mod volume;

use volume::VolumeResponse;

use crate::optional_setting;

const PROVIDER_ID: &str = lunu_core::consts::metadata::GOOGLE_BOOKS_PROVIDER;
const REQUEST_TIMEOUT_SECS: u64 = 15;
const ACCEPTS: &[IdScheme] = &[IdScheme::Isbn];
const BASE: &str = "https://www.googleapis.com/books/v1/volumes";
const SEARCH_LIMIT: &str = "10";
const LOOKUP_LIMIT: &str = "5";

pub struct GoogleBooksProvider {
	client: reqwest::Client,
	settings: Arc<SettingsService>,
}

impl GoogleBooksProvider {
	pub fn new(settings: Arc<SettingsService>) -> Self {
		let client = crate::http_client_builder(Duration::from_secs(REQUEST_TIMEOUT_SECS))
			.build()
			.expect("reqwest client builds with static configuration");

		Self { client, settings }
	}

	async fn query(&self, q: &str, max_results: &str, region: &str) -> Result<Vec<Book>> {
		let key = optional_setting(&self.settings, METADATA_GOOGLE_BOOKS_API_KEY).await?;
		let country = country_code(region);
		let body: VolumeResponse = crate::http::get_json(|| {
			let request = self.client.get(BASE).query(&[
				("q", q),
				("maxResults", max_results),
				("printType", "books"),
				("country", country.as_str()),
			]);
			match &key {
				Some(key) => request.query(&[("key", key.as_str())]),
				None => request,
			}
		})
		.await?;
		Ok(body.into_books())
	}
}

#[async_trait]
impl MetadataProvider for GoogleBooksProvider {
	fn id(&self) -> &'static str {
		PROVIDER_ID
	}

	fn accepts(&self) -> &[IdScheme] {
		ACCEPTS
	}

	async fn search(&self, query: &str, region: &str, _page: i64) -> Result<Vec<Book>> {
		self.query(query, SEARCH_LIMIT, region).await
	}

	async fn get_book(&self, id: &ExternalId, region: &str) -> Result<Option<Book>> {
		let Some(isbn) = id.value_for(IdScheme::Isbn) else {
			return Ok(None);
		};
		let wanted = ExternalId::isbn(isbn);
		let book = self
			.query(&format!("isbn:{isbn}"), LOOKUP_LIMIT, region)
			.await?
			.into_iter()
			.find(|book| book.ids.contains(&wanted));
		Ok(book)
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

fn country_code(region: &str) -> String {
	match region {
		"uk" => "GB".to_string(),
		other => other.to_ascii_uppercase(),
	}
}
