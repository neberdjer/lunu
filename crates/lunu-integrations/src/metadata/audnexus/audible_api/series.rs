use std::collections::HashMap;

use lunu_core::Result;
use lunu_core::models::{Book, SeriesRef, SeriesSummary};

use super::{RESPONSE_GROUPS, catalog_search};

const SERIES_GATHER_PAGES: i64 = 5;
const PAGE_SIZE: i64 = 50;

pub(crate) async fn search_series(
	client: &reqwest::Client,
	region: &str,
	query: &str,
) -> Result<Vec<SeriesSummary>> {
	let books = catalog_search(
		client,
		region,
		&[
			("num_results", "20"),
			("products_sort_by", "Relevance"),
			("response_groups", RESPONSE_GROUPS),
			("keywords", query),
		],
	)
	.await?;
	Ok(summarize_series(books))
}

pub(crate) async fn series_books(
	client: &reqwest::Client,
	region: &str,
	name: &str,
	asin: Option<&str>,
) -> Result<Vec<Book>> {
	let mut collected: Vec<Book> = Vec::new();
	let mut seen = std::collections::HashSet::new();
	let page_size = PAGE_SIZE.to_string();

	for page_num in 1..=SERIES_GATHER_PAGES {
		let page = page_num.to_string();
		let books = catalog_search(
			client,
			region,
			&[
				("num_results", page_size.as_str()),
				("page", page.as_str()),
				("products_sort_by", "Relevance"),
				("response_groups", RESPONSE_GROUPS),
				("keywords", name),
			],
		)
		.await?;
		let count = books.len();
		for book in books {
			if in_series(&book, name, asin) && seen.insert(book.asin.clone()) {
				collected.push(book);
			}
		}
		if count < PAGE_SIZE as usize {
			break;
		}
	}

	collected.sort_by(|a, b| position(a, name, asin).total_cmp(&position(b, name, asin)));
	Ok(collected)
}

fn matches(entry: &SeriesRef, name: &str, asin: Option<&str>) -> bool {
	match asin {
		Some(asin) => entry.asin.as_deref() == Some(asin),
		None => entry.name.eq_ignore_ascii_case(name.trim()),
	}
}

fn in_series(book: &Book, name: &str, asin: Option<&str>) -> bool {
	book.series.iter().any(|entry| matches(entry, name, asin))
}

fn position(book: &Book, name: &str, asin: Option<&str>) -> f64 {
	book.series
		.iter()
		.find(|entry| matches(entry, name, asin))
		.and_then(|entry| entry.position.as_deref())
		.and_then(parse_position)
		.unwrap_or(f64::MAX)
}

fn parse_position(position: &str) -> Option<f64> {
	position
		.split(|c: char| !c.is_ascii_digit() && c != '.')
		.find(|token| !token.is_empty())
		.and_then(|token| token.parse::<f64>().ok())
}

fn summarize_series(books: Vec<Book>) -> Vec<SeriesSummary> {
	let mut order: Vec<String> = Vec::new();
	let mut summaries: HashMap<String, SeriesSummary> = HashMap::new();

	for book in books {
		let author = book.authors.first().cloned();
		let cover_url = book.cover_url.clone();
		for entry in book.series {
			let key = entry.name.trim().to_ascii_lowercase();
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
	use super::*;

	fn book_in_series(asin: &str, series: &str, position: Option<&str>) -> Book {
		Book {
			asin: asin.to_string(),
			title: asin.to_string(),
			subtitle: None,
			authors: vec!["Isaac Asimov".to_string()],
			author_asins: Vec::new(),
			narrators: Vec::new(),
			series: vec![SeriesRef {
				name: series.to_string(),
				position: position.map(str::to_string),
				asin: Some("S1".to_string()),
			}],
			description: None,
			cover_url: Some(format!("cover-{asin}")),
			release_date: None,
			runtime_minutes: None,
			language: None,
			publisher: None,
			genres: Vec::new(),
		}
	}

	#[test]
	fn parses_series_positions() {
		assert_eq!(parse_position("1"), Some(1.0));
		assert_eq!(parse_position("1.5"), Some(1.5));
		assert_eq!(parse_position("Book 2"), Some(2.0));
		assert_eq!(parse_position("1-3"), Some(1.0));
		assert_eq!(parse_position(""), None);
	}

	#[test]
	fn sort_by_position_unpositioned_last() {
		let mut books = [
			book_in_series("b3", "Foundation", Some("3")),
			book_in_series("b1", "Foundation", Some("1")),
			book_in_series("bx", "Foundation", None),
			book_in_series("b2", "Foundation", Some("2")),
		];
		books.sort_by(|a, b| {
			position(a, "Foundation", None).total_cmp(&position(b, "Foundation", None))
		});
		let order: Vec<&str> = books.iter().map(|b| b.asin.as_str()).collect();
		assert_eq!(order, ["b1", "b2", "b3", "bx"]);
	}

	#[test]
	fn matches_by_asin_over_name() {
		let book = book_in_series("b1", "Foundation", Some("1"));
		assert!(in_series(&book, "wrong name", Some("S1")));
		assert!(!in_series(&book, "Foundation", Some("S2")));
	}

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
}
