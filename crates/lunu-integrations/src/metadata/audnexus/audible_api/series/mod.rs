use std::collections::HashMap;

use lunu_core::Result;
use lunu_core::models::Book;

use super::{books_by_asins, relationships};

mod keyword;
mod summary;

pub(crate) use summary::search_series;

const UNABRIDGED: &str = "unabridged";

pub(crate) async fn series_books(
	client: &reqwest::Client,
	region: &str,
	name: &str,
	asin: Option<&str>,
) -> Result<Vec<Book>> {
	if let Some(series_asin) = summary::resolve_series_asin(client, region, name, asin).await? {
		let books = from_relationships(client, region, &series_asin).await?;
		if !books.is_empty() {
			return Ok(books);
		}
	}
	keyword::by_keyword(client, region, name, asin).await
}

async fn from_relationships(
	client: &reqwest::Client,
	region: &str,
	series_asin: &str,
) -> Result<Vec<Book>> {
	let mut children = relationships::series_children(client, region, series_asin).await?;
	if children.is_empty() {
		return Ok(Vec::new());
	}
	children.sort_by_key(relationships::Relationship::slot);

	let asins: Vec<String> = children.iter().map(|child| child.asin.clone()).collect();
	let mut hydrated: HashMap<String, Book> = books_by_asins(client, region, &asins)
		.await?
		.into_iter()
		.map(|book| (book.asin.clone(), book))
		.collect();

	let mut slots: Vec<(u32, Book)> = Vec::new();
	for child in children {
		let Some(book) = hydrated.remove(&child.asin) else {
			continue;
		};
		let slot = child.slot();
		match slots.iter_mut().find(|(taken, _)| *taken == slot) {
			Some((_, held)) => {
				if edition_rank(&book) > edition_rank(held) {
					*held = book;
				}
			}
			None => slots.push((slot, book)),
		}
	}

	Ok(slots.into_iter().map(|(_, book)| book).collect())
}

fn edition_rank(book: &Book) -> (bool, i64) {
	(
		book.format_type
			.as_deref()
			.is_some_and(|format| format.eq_ignore_ascii_case(UNABRIDGED)),
		book.runtime_minutes.unwrap_or(0),
	)
}

#[cfg(test)]
pub(super) mod tests {
	use super::*;
	use lunu_core::models::SeriesRef;

	pub(crate) fn book_in_series(asin: &str, series: &str, position: Option<&str>) -> Book {
		book_with(asin, series, position, Some("S1"))
	}

	pub(crate) fn book_with(
		asin: &str,
		series: &str,
		position: Option<&str>,
		series_asin: Option<&str>,
	) -> Book {
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
				asin: series_asin.map(str::to_string),
			}],
			description: None,
			cover_url: Some(format!("cover-{asin}")),
			release_date: None,
			runtime_minutes: None,
			language: None,
			publisher: None,
			genres: Vec::new(),
			tags: Vec::new(),
			isbn: None,
			format_type: None,
			rating: None,
			is_adult: None,
		}
	}

	fn edition(asin: &str, format: Option<&str>, runtime: i64) -> Book {
		let mut book = book_in_series(asin, "The Lord of the Rings", Some("0.5"));
		book.format_type = format.map(str::to_string);
		book.runtime_minutes = Some(runtime);
		book
	}

	#[test]
	fn the_unabridged_edition_wins_a_slot() {
		let abridged = edition("a", Some("abridged"), 900);
		let unabridged = edition("u", Some("unabridged"), 625);
		assert!(
			edition_rank(&unabridged) > edition_rank(&abridged),
			"a longer abridged edition must not displace the unabridged one"
		);
	}

	#[test]
	fn the_longest_edition_wins_within_the_same_format() {
		assert!(
			edition_rank(&edition("a", Some("unabridged"), 662))
				> edition_rank(&edition("b", Some("unabridged"), 625))
		);
		assert!(
			edition_rank(&edition("a", Some("unabridged"), 1))
				> edition_rank(&edition("b", None, 900))
		);
	}
}
