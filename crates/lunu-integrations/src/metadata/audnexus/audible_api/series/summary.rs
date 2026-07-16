use std::collections::HashMap;

use lunu_core::Result;
use lunu_core::models::{Book, SeriesRef, SeriesSummary};

use super::keyword::keyword_page;

const SEARCH_LIMIT: i64 = 20;

pub(crate) async fn search_series(
	client: &reqwest::Client,
	region: &str,
	query: &str,
) -> Result<Vec<SeriesSummary>> {
	let books = keyword_page(client, region, query, 1, SEARCH_LIMIT).await?;
	Ok(summarize_series(books))
}

pub(super) async fn resolve_series_asin(
	client: &reqwest::Client,
	region: &str,
	name: &str,
	asin: Option<&str>,
) -> Result<Option<String>> {
	if let Some(asin) = asin {
		return Ok(Some(asin.to_string()));
	}

	let books = keyword_page(client, region, name, 1, SEARCH_LIMIT).await?;
	Ok(books
		.iter()
		.flat_map(|book| &book.series)
		.find(|entry| entry.name.eq_ignore_ascii_case(name.trim()) && entry.asin.is_some())
		.and_then(|entry| entry.asin.clone()))
}

fn series_key(entry: &SeriesRef) -> String {
	entry
		.asin
		.clone()
		.unwrap_or_else(|| entry.name.trim().to_ascii_lowercase())
}

fn summarize_series(books: Vec<Book>) -> Vec<SeriesSummary> {
	let mut order: Vec<String> = Vec::new();
	let mut summaries: HashMap<String, SeriesSummary> = HashMap::new();

	for book in books {
		let author = book.authors.first().cloned();
		let cover_url = book.cover_url.clone();
		for entry in book.series {
			let key = series_key(&entry);
			summaries
				.entry(key.clone())
				.and_modify(|summary| summary.books_in_results += 1)
				.or_insert_with(|| {
					order.push(key.clone());
					SeriesSummary {
						name: entry.name,
						asin: entry.asin,
						author: author.clone(),
						cover_url: cover_url.clone(),
						books_in_results: 1,
					}
				});
		}
	}

	order
		.into_iter()
		.filter_map(|key| summaries.remove(&key))
		.collect()
}

#[cfg(test)]
mod tests {
	use super::super::tests::{book_in_series, book_with};
	use super::*;

	#[test]
	fn summarize_aggregates_author_cover_count() {
		let books = vec![
			book_in_series("b1", "Foundation", Some("1")),
			book_in_series("b2", "Foundation", Some("2")),
		];
		let series = summarize_series(books);
		assert_eq!(series.len(), 1);
		assert_eq!(series[0].name, "Foundation");
		assert_eq!(series[0].books_in_results, 2);
		assert_eq!(series[0].author.as_deref(), Some("Isaac Asimov"));
		assert_eq!(series[0].cover_url.as_deref(), Some("cover-b1"));
	}

	#[test]
	fn two_series_sharing_a_name_stay_separate() {
		let books = vec![
			book_with("b1", "The Lord of the Rings", Some("1"), Some("B009CFOEGK")),
			book_with("b2", "The Lord of the Rings", Some("1"), Some("B0DNN628VN")),
		];
		let series = summarize_series(books);
		assert_eq!(
			series.len(),
			2,
			"distinct series asins must not merge just because audible reuses a title"
		);
		assert_eq!(series[0].books_in_results, 1);
		assert_eq!(series[1].books_in_results, 1);
	}

	#[test]
	fn series_without_asins_still_group_by_name() {
		let books = vec![
			book_with("b1", "Foundation", Some("1"), None),
			book_with("b2", "foundation", Some("2"), None),
		];
		let series = summarize_series(books);
		assert_eq!(series.len(), 1);
		assert_eq!(series[0].books_in_results, 2);
	}
}
