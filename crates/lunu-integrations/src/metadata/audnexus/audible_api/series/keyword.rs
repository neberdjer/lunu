use std::collections::HashSet;

use lunu_core::Result;
use lunu_core::models::{Book, SeriesRef};

use super::super::{audible_page, catalog_params, catalog_search};

const SERIES_GATHER_PAGES: i64 = 5;
const PAGE_SIZE: i64 = 50;

pub(super) async fn keyword_page(
	client: &reqwest::Client,
	region: &str,
	query: &str,
	page: i64,
	limit: i64,
) -> Result<Vec<Book>> {
	let page = audible_page(page);
	let limit = limit.to_string();
	catalog_search(
		client,
		region,
		&catalog_params(&[
			("num_results", limit.as_str()),
			("page", page.as_str()),
			("products_sort_by", "Relevance"),
			("keywords", query),
		]),
	)
	.await
}

pub(super) async fn by_keyword(
	client: &reqwest::Client,
	region: &str,
	name: &str,
	asin: Option<&str>,
) -> Result<Vec<Book>> {
	let mut collected: Vec<Book> = Vec::new();
	let mut seen = HashSet::new();

	for page in 1..=SERIES_GATHER_PAGES {
		let books = keyword_page(client, region, name, page, PAGE_SIZE).await?;
		let count = books.len();
		for book in books {
			if in_series(&book, name, asin)
				&& book
					.asin()
					.is_some_and(|asin| seen.insert(asin.to_string()))
			{
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

#[cfg(test)]
mod tests {
	use super::super::tests::book_in_series;
	use super::*;

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
		let order: Vec<&str> = books.iter().filter_map(|b| b.asin()).collect();
		assert_eq!(order, ["b1", "b2", "b3", "bx"]);
	}

	#[test]
	fn matches_by_asin_over_name() {
		let book = book_in_series("b1", "Foundation", Some("1"));
		assert!(in_series(&book, "wrong name", Some("S1")));
		assert!(!in_series(&book, "Foundation", Some("S2")));
	}
}
