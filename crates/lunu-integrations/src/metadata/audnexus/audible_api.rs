use std::collections::HashMap;

use lunu_core::Result;
use lunu_core::models::{Book, SeriesRef};
use serde::Deserialize;

use super::{Named, asins, names};
use crate::http::send_with_retry;
use crate::integration_error;

const SEARCH_RESULT_LIMIT: &str = "10";
const RESPONSE_GROUPS: &str = "contributors,product_desc,product_attrs,media,series";

fn to_books(products: Vec<AudibleProduct>) -> Vec<Book> {
	products
		.into_iter()
		.map(AudibleProduct::into_book)
		.collect()
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
	Ok(to_books(body.products))
}

pub(super) async fn search(
	client: &reqwest::Client,
	region: &str,
	query: &str,
	page: i64,
) -> Result<Vec<Book>> {
	let page = page.max(1).to_string();
	catalog_search(
		client,
		region,
		&[
			("num_results", SEARCH_RESULT_LIMIT),
			("page", page.as_str()),
			("products_sort_by", "Relevance"),
			("response_groups", RESPONSE_GROUPS),
			("keywords", query),
		],
	)
	.await
}

pub(super) async fn books_by_author(
	client: &reqwest::Client,
	region: &str,
	author_asin: &str,
) -> Result<Vec<Book>> {
	catalog_search(
		client,
		region,
		&[
			("num_results", "20"),
			("products_sort_by", "Relevance"),
			("response_groups", RESPONSE_GROUPS),
			("author", author_asin),
		],
	)
	.await
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
		client.get(&url).query(&[
			("response_groups", RESPONSE_GROUPS),
			("similarity_type", "ByTheSameAuthor"),
		])
	})
	.await?
	.error_for_status()
	.map_err(integration_error)?;

	let body: AudibleSimilarResponse = response.json().await.map_err(integration_error)?;
	Ok(to_books(body.similar_products))
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
	#[serde(default)]
	products: Vec<AudibleProduct>,
}

#[derive(Deserialize)]
struct AudibleSimilarResponse {
	#[serde(default)]
	similar_products: Vec<AudibleProduct>,
}

#[derive(Deserialize)]
struct AudibleSeries {
	title: String,
	sequence: Option<String>,
	asin: Option<String>,
}

#[derive(Deserialize)]
struct AudibleProduct {
	asin: String,
	title: String,
	subtitle: Option<String>,
	#[serde(default)]
	authors: Vec<Named>,
	#[serde(default)]
	narrators: Vec<Named>,
	#[serde(default)]
	series: Vec<AudibleSeries>,
	merchandising_summary: Option<String>,
	product_images: Option<HashMap<String, String>>,
	release_date: Option<String>,
	runtime_length_min: Option<i64>,
	language: Option<String>,
	publisher_name: Option<String>,
}

impl AudibleProduct {
	fn into_book(self) -> Book {
		Book {
			asin: self.asin,
			title: self.title,
			subtitle: self.subtitle,
			authors: names(&self.authors),
			author_asins: asins(&self.authors),
			narrators: names(&self.narrators),
			series: self
				.series
				.into_iter()
				.map(|entry| SeriesRef {
					name: entry.title,
					position: entry.sequence,
					asin: entry.asin,
				})
				.collect(),
			description: self.merchandising_summary,
			cover_url: largest_image(self.product_images),
			release_date: self.release_date,
			runtime_minutes: self.runtime_length_min,
			language: self.language,
			publisher: self.publisher_name,
			genres: Vec::new(),
		}
	}
}

fn largest_image(images: Option<HashMap<String, String>>) -> Option<String> {
	images.and_then(|images| {
		images
			.into_iter()
			.max_by_key(|(size, _)| size.parse::<u32>().unwrap_or(0))
			.map(|(_, url)| url)
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	const AUDIBLE_SEARCH: &str = r#"{
		"products": [{
			"asin": "B0036I54I6",
			"title": "The Hobbit",
			"authors": [{"name": "J.R.R. Tolkien"}],
			"narrators": [{"name": "Rob Inglis"}],
			"series": [{"title": "The Lord of the Rings", "sequence": "0"}],
			"merchandising_summary": "A great adventure.",
			"product_images": {"500": "https://images/500.jpg", "1024": "https://images/1024.jpg"},
			"release_date": "2012-01-01",
			"runtime_length_min": 662,
			"language": "english",
			"publisher_name": "Recorded Books"
		}],
		"total_results": 1
	}"#;

	#[test]
	fn parses_audible_search() {
		let response: AudibleSearchResponse = serde_json::from_str(AUDIBLE_SEARCH).unwrap();
		let books: Vec<Book> = response
			.products
			.into_iter()
			.map(AudibleProduct::into_book)
			.collect();
		assert_eq!(books.len(), 1);
		assert_eq!(books[0].title, "The Hobbit");
		assert_eq!(
			books[0].cover_url.as_deref(),
			Some("https://images/1024.jpg")
		);
		assert_eq!(books[0].series[0].position.as_deref(), Some("0"));
	}
}
