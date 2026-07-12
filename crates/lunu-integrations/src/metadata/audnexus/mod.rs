use std::time::Duration;

use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::models::{Book, Chapters};
use lunu_core::traits::MetadataProvider;
use serde::Deserialize;

mod audible_api;
mod audnex_api;

const PROVIDER_ID: &str = "audnexus";
const REQUEST_TIMEOUT_SECS: u64 = 15;

pub struct AudnexusProvider {
	client: reqwest::Client,
}

impl AudnexusProvider {
	pub fn new() -> Self {
		let client = crate::http_client_builder(Duration::from_secs(REQUEST_TIMEOUT_SECS))
			.build()
			.expect("reqwest client builds with static configuration");

		Self { client }
	}
}

impl Default for AudnexusProvider {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl MetadataProvider for AudnexusProvider {
	fn id(&self) -> &'static str {
		PROVIDER_ID
	}

	async fn search(&self, query: &str, region: &str, page: i64) -> Result<Vec<Book>> {
		audible_api::search(&self.client, region, query, page).await
	}

	async fn get_book(&self, asin: &str, region: &str) -> Result<Option<Book>> {
		audnex_api::get_book(&self.client, region, asin).await
	}

	async fn get_chapters(&self, asin: &str, region: &str) -> Result<Option<Chapters>> {
		audnex_api::get_chapters(&self.client, region, asin).await
	}

	async fn similar(&self, asin: &str, region: &str) -> Result<Vec<Book>> {
		audible_api::similar(&self.client, region, asin).await
	}

	async fn books_by_author(&self, author_asin: &str, region: &str) -> Result<Vec<Book>> {
		audible_api::books_by_author(&self.client, region, author_asin).await
	}
}

#[derive(Deserialize, Clone)]
struct Named {
	name: String,
	#[serde(default)]
	asin: Option<String>,
}

fn names(items: &[Named]) -> Vec<String> {
	items.iter().map(|item| item.name.clone()).collect()
}

fn asins(items: &[Named]) -> Vec<String> {
	items.iter().filter_map(|item| item.asin.clone()).collect()
}
