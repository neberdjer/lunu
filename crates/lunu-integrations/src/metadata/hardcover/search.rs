use lunu_core::models::{Book, ExternalId};
use serde::Deserialize;

use super::{GENRE_LIMIT, format_rating};
use crate::{nonempty, to_https};

#[derive(Deserialize)]
pub(super) struct SearchData {
	search: Search,
}

impl SearchData {
	pub(super) fn into_books(self) -> Vec<Book> {
		self.search
			.results
			.hits
			.into_iter()
			.filter_map(|hit| hit.document.into_book())
			.collect()
	}
}

#[derive(Deserialize)]
struct Search {
	results: Results,
}

#[derive(Deserialize)]
struct Results {
	#[serde(default)]
	hits: Vec<Hit>,
}

#[derive(Deserialize)]
struct Hit {
	document: Document,
}

#[derive(Deserialize)]
struct Document {
	title: Option<String>,
	subtitle: Option<String>,
	#[serde(default)]
	author_names: Vec<String>,
	description: Option<String>,
	image: Option<Image>,
	#[serde(default)]
	isbns: Vec<String>,
	#[serde(default)]
	genres: Vec<String>,
	release_date: Option<String>,
	release_year: Option<i64>,
	rating: Option<f64>,
}

#[derive(Deserialize)]
struct Image {
	url: Option<String>,
}

impl Document {
	fn into_book(self) -> Option<Book> {
		let title = nonempty(self.title)?;

		Some(Book {
			ids: self.isbns.into_iter().map(ExternalId::isbn).collect(),
			title,
			subtitle: nonempty(self.subtitle),
			authors: self.author_names,
			author_asins: Vec::new(),
			narrators: Vec::new(),
			series: Vec::new(),
			description: nonempty(self.description),
			cover_url: self.image.and_then(|image| image.url).map(to_https),
			release_date: nonempty(self.release_date)
				.or_else(|| self.release_year.map(|year| year.to_string())),
			runtime_minutes: None,
			language: None,
			publisher: None,
			genres: self.genres.into_iter().take(GENRE_LIMIT).collect(),
			tags: Vec::new(),
			format_type: None,
			rating: format_rating(self.rating),
			is_adult: None,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const RESULTS: &str = r#"{
		"search": {
			"results": {
				"hits": [
					{"document": {
						"title": "The Hobbit",
						"subtitle": "There and Back Again",
						"author_names": ["J.R.R. Tolkien"],
						"description": "A hobbit's tale.",
						"image": {"url": "http://covers.hardcover.app/hobbit.jpg"},
						"isbns": ["9780007487295", "0007487290"],
						"genres": ["Fantasy", "Classics"],
						"release_year": 1937,
						"rating": 4.31
					}},
					{"document": {"author_names": ["Nobody"]}}
				]
			}
		}
	}"#;

	fn books() -> Vec<Book> {
		serde_json::from_str::<SearchData>(RESULTS)
			.unwrap()
			.into_books()
	}

	#[test]
	fn a_hit_maps_the_denormalized_document_and_drops_a_titleless_one() {
		let books = books();
		assert_eq!(
			books.len(),
			1,
			"a document the search omitted a title for is skipped"
		);
		let book = &books[0];
		assert_eq!(book.title, "The Hobbit");
		assert_eq!(book.authors, vec!["J.R.R. Tolkien"]);
		assert_eq!(
			book.ids,
			vec![
				ExternalId::isbn("9780007487295"),
				ExternalId::isbn("0007487290"),
			]
		);
		assert_eq!(book.genres, vec!["Fantasy", "Classics"]);
		assert_eq!(book.rating.as_deref(), Some("4.31"));
	}

	#[test]
	fn the_release_year_stands_in_for_a_missing_date_and_the_cover_upgrades_to_https() {
		let book = &books()[0];
		assert_eq!(book.release_date.as_deref(), Some("1937"));
		assert_eq!(
			book.cover_url.as_deref(),
			Some("https://covers.hardcover.app/hobbit.jpg")
		);
	}
}
