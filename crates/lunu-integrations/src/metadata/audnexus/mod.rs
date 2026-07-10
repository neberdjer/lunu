use std::time::Duration;

use async_trait::async_trait;
use lunu_core::models::{Book, Chapters};
use lunu_core::traits::MetadataProvider;
use lunu_core::{Error, Result};
use serde::Deserialize;

mod audible_api;
mod audnex_api;

const PROVIDER_ID: &str = "audnexus";
const DEFAULT_REGION: &str = "us";
const REQUEST_TIMEOUT_SECS: u64 = 15;

pub struct AudnexusProvider {
	client: reqwest::Client,
	region: String,
}

impl AudnexusProvider {
	pub fn new(region: impl Into<String>) -> Self {
		let client = reqwest::Client::builder()
			.user_agent(concat!("lunu/", env!("CARGO_PKG_VERSION")))
			.timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
			.build()
			.expect("reqwest client builds with static configuration");

		Self {
			client,
			region: region.into(),
		}
	}

	pub fn with_default_region() -> Self {
		Self::new(DEFAULT_REGION)
	}
}

#[async_trait]
impl MetadataProvider for AudnexusProvider {
	fn id(&self) -> &'static str {
		PROVIDER_ID
	}

	async fn search(&self, query: &str) -> Result<Vec<Book>> {
		audible_api::search(&self.client, &self.region, query).await
	}

	async fn get_book(&self, asin: &str) -> Result<Option<Book>> {
		audnex_api::get_book(&self.client, &self.region, asin).await
	}

	async fn get_chapters(&self, asin: &str) -> Result<Option<Chapters>> {
		audnex_api::get_chapters(&self.client, &self.region, asin).await
	}
}

fn integration_error(error: impl std::fmt::Display) -> Error {
	Error::Integration(error.to_string())
}

#[derive(Deserialize)]
struct Named {
	name: String,
}

fn names(items: Vec<Named>) -> Vec<String> {
	items.into_iter().map(|item| item.name).collect()
}
