use lunu_core::Result;
use lunu_core::models::{Book, ExternalId};
use serde::Deserialize;

use super::{BASE, GENRE_LIMIT};
use crate::http::get_json;

const SEARCH_RESULT_LIMIT: &str = "10";
const SEARCH_FIELDS: &str = "title,author_name,first_publish_year,isbn,cover_i,publisher,subject";

pub(super) async fn search(client: &reqwest::Client, query: &str, page: i64) -> Result<Vec<Book>> {
	let page = page.max(1).to_string();
	let url = format!("{BASE}/search.json");
	let body: SearchResponse = get_json(|| {
		client.get(&url).query(&[
			("q", query),
			("limit", SEARCH_RESULT_LIMIT),
			("page", page.as_str()),
			("fields", SEARCH_FIELDS),
		])
	})
	.await?;
	Ok(body.docs.into_iter().map(SearchDoc::into_book).collect())
}

#[derive(Deserialize)]
struct SearchResponse {
	#[serde(default)]
	docs: Vec<SearchDoc>,
}

#[derive(Deserialize)]
struct SearchDoc {
	title: String,
	#[serde(default)]
	author_name: Vec<String>,
	first_publish_year: Option<i64>,
	#[serde(default)]
	isbn: Vec<String>,
	cover_i: Option<i64>,
	#[serde(default)]
	publisher: Vec<String>,
	#[serde(default)]
	subject: Vec<String>,
}

impl SearchDoc {
	fn into_book(self) -> Book {
		Book {
			ids: best_isbn(&self.isbn)
				.map(ExternalId::isbn)
				.into_iter()
				.collect(),
			title: self.title,
			subtitle: None,
			authors: self.author_name,
			author_asins: Vec::new(),
			narrators: Vec::new(),
			series: Vec::new(),
			description: None,
			cover_url: self.cover_i.map(cover_url),
			release_date: self.first_publish_year.map(|year| year.to_string()),
			runtime_minutes: None,
			language: None,
			publisher: self.publisher.into_iter().next(),
			genres: self.subject.into_iter().take(GENRE_LIMIT).collect(),
			tags: Vec::new(),
			format_type: None,
			rating: None,
			is_adult: None,
		}
	}
}

fn best_isbn(isbns: &[String]) -> Option<String> {
	isbns
		.iter()
		.find(|isbn| isbn.len() == 13)
		.or_else(|| isbns.first())
		.cloned()
}

fn cover_url(cover_id: i64) -> String {
	format!("https://covers.openlibrary.org/b/id/{cover_id}-L.jpg")
}

#[cfg(test)]
mod tests {
	use super::*;

	const SEARCH: &str = r#"{
		"numFound": 194,
		"docs": [{
			"title": "The Hobbit",
			"author_name": ["J.R.R. Tolkien"],
			"first_publish_year": 1937,
			"isbn": ["0201111586", "9780007487295", "0008108285"],
			"cover_i": 14627509,
			"publisher": ["HarperCollins", "Houghton Mifflin"],
			"subject": ["Fantasy", "Arkenstone", "Battle of Five Armies"]
		}, {
			"title": "Sparse Result"
		}]
	}"#;

	fn books() -> Vec<Book> {
		serde_json::from_str::<SearchResponse>(SEARCH)
			.unwrap()
			.docs
			.into_iter()
			.map(SearchDoc::into_book)
			.collect()
	}

	#[test]
	fn a_thirteen_digit_isbn_is_preferred_over_edition_noise() {
		let book = &books()[0];
		assert_eq!(
			book.ids,
			vec![ExternalId::isbn("9780007487295")],
			"the isbn array lists every edition, and the first entry is arbitrary"
		);
		assert_eq!(book.title, "The Hobbit");
		assert_eq!(book.authors, vec!["J.R.R. Tolkien"]);
		assert_eq!(book.release_date.as_deref(), Some("1937"));
		assert_eq!(
			book.cover_url.as_deref(),
			Some("https://covers.openlibrary.org/b/id/14627509-L.jpg")
		);
		assert_eq!(book.publisher.as_deref(), Some("HarperCollins"));
	}

	#[test]
	fn a_doc_without_isbns_yields_a_book_with_no_ids() {
		let book = &books()[1];
		assert!(
			book.ids.is_empty(),
			"a book this source cannot identify must not invent an identifier"
		);
		assert_eq!(book.title, "Sparse Result");
	}
}
