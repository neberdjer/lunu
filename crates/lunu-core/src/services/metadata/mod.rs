use std::sync::Arc;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::consts::metadata::{
	DEFAULT_METADATA_REGION, DEFAULT_PROVIDER_PRIORITY, METADATA_REGION_SETTING,
	VALID_METADATA_REGIONS, provider_settings,
};
use crate::consts::reasons;
use crate::consts::settings::TOGGLE_ON;
use crate::models::{Book, Chapters, ExternalId, IdScheme, SeriesSummary};
use crate::repo::MetadataCacheRepo;
use crate::services::SettingsService;
use crate::traits::MetadataProvider;
use crate::{Error, Result};

mod cache;

const KIND_SEARCH: &str = "search";
const KIND_BOOK: &str = "book";
const KIND_CHAPTERS: &str = "chapters";
const KIND_SIMILAR: &str = "similar";
const KIND_AUTHOR: &str = "author";
const KIND_SERIES_SEARCH: &str = "series-search";
const KIND_SERIES_BOOKS: &str = "series-books";

pub struct MetadataService {
	providers: Vec<Arc<dyn MetadataProvider>>,
	cache: Arc<dyn MetadataCacheRepo>,
	settings: Arc<SettingsService>,
}

impl MetadataService {
	pub fn new(
		providers: Vec<Arc<dyn MetadataProvider>>,
		cache: Arc<dyn MetadataCacheRepo>,
		settings: Arc<SettingsService>,
	) -> Self {
		Self {
			providers,
			cache,
			settings,
		}
	}

	async fn enabled_providers(
		&self,
		scheme: Option<IdScheme>,
	) -> Result<Vec<&Arc<dyn MetadataProvider>>> {
		let candidates: Vec<_> = self
			.providers
			.iter()
			.enumerate()
			.filter(|(_, provider)| {
				scheme.is_none_or(|scheme| provider.accepts().contains(&scheme))
			})
			.collect();

		let keys: Vec<&str> = candidates
			.iter()
			.filter_map(|(_, provider)| provider_settings(provider.id()))
			.flat_map(|settings| [settings.enabled, settings.priority])
			.collect();
		let resolved = self.settings.resolve_many(&keys).await?;

		let mut ranked = Vec::new();
		for (registered, provider) in candidates {
			let Some(settings) = provider_settings(provider.id()) else {
				ranked.push((DEFAULT_PROVIDER_PRIORITY, registered, provider));
				continue;
			};
			if resolved.get(settings.enabled).map(String::as_str) != Some(TOGGLE_ON) {
				continue;
			}
			let priority = resolved
				.get(settings.priority)
				.and_then(|value| value.trim().parse().ok())
				.unwrap_or(DEFAULT_PROVIDER_PRIORITY);
			ranked.push((priority, registered, provider));
		}

		if ranked.is_empty() {
			let reason = match scheme {
				Some(_) => reasons::NO_PROVIDER_FOR_ID,
				None => reasons::NO_METADATA_PROVIDER,
			};
			return Err(Error::Validation(reason.to_string()));
		}

		ranked.sort_by_key(|(priority, registered, _)| (*priority, *registered));
		Ok(ranked
			.into_iter()
			.map(|(_, _, provider)| provider)
			.collect())
	}

	async fn fetch<T>(
		&self,
		scheme: Option<IdScheme>,
		kind: &str,
		key: &str,
		call: impl AsyncFn(&dyn MetadataProvider) -> Result<Option<T>>,
	) -> Result<Option<T>>
	where
		T: Serialize + DeserializeOwned,
	{
		let providers = self.enabled_providers(scheme).await?;

		let mut last_error = None;
		for provider in providers {
			if let Some(hit) = self.read_cache::<T>(provider.id(), kind, key).await? {
				return Ok(Some(hit));
			}
			match call(provider.as_ref()).await {
				Ok(Some(value)) => {
					self.write_cache(provider.id(), kind, key, &value).await?;
					return Ok(Some(value));
				}
				Ok(None) => {}
				Err(error) => last_error = Some(error),
			}
		}

		match last_error {
			Some(error) => Err(error),
			None => Ok(None),
		}
	}

	async fn fetch_list<T>(
		&self,
		scheme: Option<IdScheme>,
		kind: &str,
		key: &str,
		call: impl AsyncFn(&dyn MetadataProvider) -> Result<Vec<T>>,
	) -> Result<Vec<T>>
	where
		T: Serialize + DeserializeOwned,
	{
		let found = self
			.fetch::<Vec<T>>(scheme, kind, key, async |provider| {
				let items = call(provider).await?;
				Ok((!items.is_empty()).then_some(items))
			})
			.await?;
		Ok(found.unwrap_or_default())
	}

	pub async fn search(&self, query: &str, page: i64) -> Result<Vec<Book>> {
		let normalized = query.trim().to_lowercase();
		if normalized.is_empty() {
			return Ok(Vec::new());
		}

		let page = page.max(1);
		let region = self.region().await?;
		let cache_key = format!("{region}:{page}:{normalized}");

		self.fetch_list(None, KIND_SEARCH, &cache_key, async |provider| {
			provider.search(query, &region, page).await
		})
		.await
	}

	pub async fn search_series(&self, query: &str) -> Result<Vec<SeriesSummary>> {
		let normalized = query.trim().to_lowercase();
		if normalized.is_empty() {
			return Ok(Vec::new());
		}

		let region = self.region().await?;
		let cache_key = format!("{region}:{normalized}");

		self.fetch_list(None, KIND_SERIES_SEARCH, &cache_key, async |provider| {
			provider.search_series(query, &region).await
		})
		.await
	}

	pub async fn series_books(&self, name: &str, id: Option<&ExternalId>) -> Result<Vec<Book>> {
		let normalized = name.trim().to_lowercase();
		if normalized.is_empty() && id.is_none() {
			return Ok(Vec::new());
		}

		let region = self.region().await?;
		let series = id.map(ExternalId::to_string).unwrap_or_default();
		let cache_key = format!("{region}:{series}:{normalized}");

		self.fetch_list(
			id.map(|id| id.scheme),
			KIND_SERIES_BOOKS,
			&cache_key,
			async |provider| provider.series_books(name, id, &region).await,
		)
		.await
	}

	pub async fn similar(&self, id: &ExternalId) -> Result<Vec<Book>> {
		let region = self.region().await?;
		let cache_key = format!("{region}:{id}");

		self.fetch_list(
			Some(id.scheme),
			KIND_SIMILAR,
			&cache_key,
			async |provider| provider.similar(id, &region).await,
		)
		.await
	}

	pub async fn books_by_author(&self, author: &ExternalId) -> Result<Vec<Book>> {
		let region = self.region().await?;
		let cache_key = format!("{region}:{author}");

		self.fetch_list(
			Some(author.scheme),
			KIND_AUTHOR,
			&cache_key,
			async |provider| provider.books_by_author(author, &region).await,
		)
		.await
	}

	pub async fn get_book(&self, id: &ExternalId) -> Result<Option<Book>> {
		let region = self.region().await?;
		let cache_key = format!("{region}:{id}");

		self.fetch(Some(id.scheme), KIND_BOOK, &cache_key, async |provider| {
			provider.get_book(id, &region).await
		})
		.await
	}

	pub async fn get_chapters(&self, id: &ExternalId) -> Result<Option<Chapters>> {
		let region = self.region().await?;
		let cache_key = format!("{region}:{id}");

		self.fetch(
			Some(id.scheme),
			KIND_CHAPTERS,
			&cache_key,
			async |provider| provider.get_chapters(id, &region).await,
		)
		.await
	}

	async fn region(&self) -> Result<String> {
		let region = self
			.settings
			.get(METADATA_REGION_SETTING)
			.await?
			.map(|value| value.trim().to_ascii_lowercase())
			.filter(|value| !value.is_empty())
			.unwrap_or_else(|| DEFAULT_METADATA_REGION.to_string());

		if !VALID_METADATA_REGIONS.contains(&region.as_str()) {
			return Err(Error::Validation(reasons::INVALID_REGION.to_string()));
		}

		Ok(region)
	}
}
