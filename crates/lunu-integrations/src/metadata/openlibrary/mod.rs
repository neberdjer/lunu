use std::time::Duration;

use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::models::{Book, Chapters, ExternalId, IdScheme, SeriesSummary};
use lunu_core::traits::MetadataProvider;

mod book;
mod search;

const PROVIDER_ID: &str = lunu_core::consts::metadata::OPENLIBRARY_PROVIDER;
const REQUEST_TIMEOUT_SECS: u64 = 15;
const ACCEPTS: &[IdScheme] = &[IdScheme::Isbn];
const BASE: &str = "https://openlibrary.org";
pub(super) use lunu_core::consts::metadata::METADATA_GENRE_LIMIT as GENRE_LIMIT;

pub struct OpenLibraryProvider {
	client: reqwest::Client,
}

impl OpenLibraryProvider {
	pub fn new() -> Self {
		let client = crate::http_client_builder(Duration::from_secs(REQUEST_TIMEOUT_SECS))
			.build()
			.expect("reqwest client builds with static configuration");

		Self { client }
	}
}

impl Default for OpenLibraryProvider {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl MetadataProvider for OpenLibraryProvider {
	fn id(&self) -> &'static str {
		PROVIDER_ID
	}

	fn accepts(&self) -> &[IdScheme] {
		ACCEPTS
	}

	async fn search(&self, query: &str, _region: &str, page: i64) -> Result<Vec<Book>> {
		search::search(&self.client, query, page).await
	}

	async fn get_book(&self, id: &ExternalId, _region: &str) -> Result<Option<Book>> {
		let Some(isbn) = isbn_of(id) else {
			return Ok(None);
		};
		book::by_isbn(&self.client, isbn).await
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

fn isbn_of(id: &ExternalId) -> Option<&str> {
	id.value_for(IdScheme::Isbn)
}
