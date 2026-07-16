use lunu_core::Result;
use lunu_core::models::Book;
use serde::Deserialize;

use crate::http::send_with_retry;
use crate::integration_error;

mod product;
mod relationships;
mod series;

use product::AudibleProduct;

pub(super) use relationships::series_parents;
pub(super) use series::{search_series, series_books};

const SEARCH_RESULT_LIMIT: &str = "10";
const RESPONSE_GROUPS: &str =
	"contributors,product_desc,product_extended_attrs,product_attrs,media,series,category_ladders";
const IMAGE_SIZES: &str = "500,1024,2400";
const CONTENT_TYPES: &str = "Product,Performance,Lecture,Speech";
const MAX_BATCH_ASINS: usize = 50;

fn catalog_params<'a>(extra: &[(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
	let mut params = vec![
		("response_groups", RESPONSE_GROUPS),
		("image_sizes", IMAGE_SIZES),
		("content_type", CONTENT_TYPES),
	];
	params.extend_from_slice(extra);
	params
}

fn audible_page(page: i64) -> String {
	(page.max(1) - 1).to_string()
}

async fn catalog_search(
	client: &reqwest::Client,
	region: &str,
	params: &[(&str, &str)],
) -> Result<Vec<Book>> {
	let url = format!("https://{}/1.0/catalog/products", audible_host(region));
	let response = send_with_retry(|| client.get(&url).query(params))
		.await?
		.error_for_status()
		.map_err(integration_error)?;

	let body: AudibleSearchResponse = response.json().await.map_err(integration_error)?;
	Ok(body
		.products
		.into_iter()
		.map(AudibleProduct::into_book)
		.collect())
}

pub(super) async fn search(
	client: &reqwest::Client,
	region: &str,
	query: &str,
	page: i64,
) -> Result<Vec<Book>> {
	let page = audible_page(page);
	catalog_search(
		client,
		region,
		&catalog_params(&[
			("num_results", SEARCH_RESULT_LIMIT),
			("page", page.as_str()),
			("products_sort_by", "Relevance"),
			("keywords", query),
		]),
	)
	.await
}

pub(super) async fn books_by_author(
	client: &reqwest::Client,
	region: &str,
	author_name: &str,
) -> Result<Vec<Book>> {
	catalog_search(
		client,
		region,
		&catalog_params(&[
			("num_results", "20"),
			("products_sort_by", "Relevance"),
			("author", author_name),
		]),
	)
	.await
}

async fn books_by_asins(
	client: &reqwest::Client,
	region: &str,
	asins: &[String],
) -> Result<Vec<Book>> {
	let mut books = Vec::with_capacity(asins.len());
	for chunk in asins.chunks(MAX_BATCH_ASINS) {
		let joined = chunk.join(",");
		books.extend(
			catalog_search(
				client,
				region,
				&catalog_params(&[("asins", joined.as_str())]),
			)
			.await?,
		);
	}
	Ok(books)
}

pub(super) async fn similar(
	client: &reqwest::Client,
	region: &str,
	asin: &str,
) -> Result<Vec<Book>> {
	let url = format!(
		"https://{}/1.0/catalog/products/{asin}/sims",
		audible_host(region)
	);
	let response = send_with_retry(|| {
		client
			.get(&url)
			.query(&catalog_params(&[("similarity_type", "ByTheSameAuthor")]))
	})
	.await?
	.error_for_status()
	.map_err(integration_error)?;

	let body: AudibleSimilarResponse = response.json().await.map_err(integration_error)?;
	Ok(body
		.similar_products
		.into_iter()
		.map(AudibleProduct::into_book)
		.collect())
}

fn audible_host(region: &str) -> &'static str {
	match region {
		"uk" => "api.audible.co.uk",
		"de" => "api.audible.de",
		"fr" => "api.audible.fr",
		"ca" => "api.audible.ca",
		"au" => "api.audible.com.au",
		"in" => "api.audible.in",
		"jp" => "api.audible.co.jp",
		"es" => "api.audible.es",
		"it" => "api.audible.it",
		_ => "api.audible.com",
	}
}

#[derive(Deserialize)]
struct AudibleSearchResponse {
	#[serde(default, deserialize_with = "lenient_products")]
	products: Vec<AudibleProduct>,
}

#[derive(Deserialize)]
struct AudibleSimilarResponse {
	#[serde(default, deserialize_with = "lenient_products")]
	similar_products: Vec<AudibleProduct>,
}

fn lenient_products<'de, D>(deserializer: D) -> std::result::Result<Vec<AudibleProduct>, D::Error>
where
	D: serde::Deserializer<'de>,
{
	let values = Vec::<serde_json::Value>::deserialize(deserializer)?;
	Ok(values
		.into_iter()
		.filter_map(|value| serde_json::from_value(value).ok())
		.collect())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn the_first_user_page_maps_to_audibles_zeroth() {
		assert_eq!(
			audible_page(1),
			"0",
			"audible pages are zero indexed, so page 1 asking for 1 skips the top results entirely"
		);
		assert_eq!(audible_page(2), "1");
	}

	#[test]
	fn pages_below_one_clamp_to_the_first() {
		assert_eq!(audible_page(0), "0");
		assert_eq!(audible_page(-5), "0");
	}

	#[test]
	fn every_request_carries_the_groups_that_populate_a_book() {
		let params = catalog_params(&[("keywords", "dune")]);
		let groups = params
			.iter()
			.find(|(key, _)| *key == "response_groups")
			.map(|(_, value)| *value)
			.expect("response_groups is always sent");

		for group in [
			"category_ladders",
			"product_extended_attrs",
			"series",
			"media",
		] {
			assert!(
				groups.contains(group),
				"{group} is missing, so that data silently never arrives"
			);
		}
		assert!(params.iter().any(|(key, _)| *key == "image_sizes"));
		assert!(params.iter().any(|(key, _)| *key == "content_type"));
		assert!(params.contains(&("keywords", "dune")));
	}

	#[test]
	fn batches_never_exceed_what_audible_accepts() {
		let asins: Vec<String> = (0..145).map(|index| format!("B{index:09}")).collect();
		let batches: Vec<usize> = asins.chunks(MAX_BATCH_ASINS).map(<[String]>::len).collect();
		assert_eq!(
			batches,
			[50, 50, 45],
			"audible rejects an asins batch over 50 with a 400"
		);
	}

	#[test]
	fn unknown_region_falls_back_to_the_us_host() {
		assert_eq!(audible_host("uk"), "api.audible.co.uk");
		assert_eq!(audible_host("zz"), "api.audible.com");
	}
}
