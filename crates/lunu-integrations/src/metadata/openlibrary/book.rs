use std::collections::HashMap;

use lunu_core::Result;
use lunu_core::models::{Book, ExternalId};
use serde::Deserialize;

use super::{BASE, GENRE_LIMIT};
use crate::http::get_json;

pub(super) async fn by_isbn(client: &reqwest::Client, isbn: &str) -> Result<Option<Book>> {
	let bibkey = format!("ISBN:{isbn}");
	let url = format!("{BASE}/api/books");
	let mut body: HashMap<String, Edition> = get_json(|| {
		client.get(&url).query(&[
			("bibkeys", bibkey.as_str()),
			("format", "json"),
			("jscmd", "data"),
		])
	})
	.await?;
	Ok(body.remove(&bibkey).map(|edition| edition.into_book(isbn)))
}

#[derive(Deserialize)]
struct Named {
	name: String,
}

#[derive(Deserialize, Default)]
struct Identifiers {
	#[serde(default)]
	isbn_13: Vec<String>,
	#[serde(default)]
	isbn_10: Vec<String>,
}

#[derive(Deserialize)]
struct Cover {
	large: Option<String>,
}

#[derive(Deserialize)]
struct Edition {
	title: String,
	#[serde(default)]
	authors: Vec<Named>,
	#[serde(default)]
	identifiers: Identifiers,
	#[serde(default)]
	publishers: Vec<Named>,
	publish_date: Option<String>,
	#[serde(default)]
	subjects: Vec<Named>,
	#[serde(default)]
	cover: Option<Cover>,
}

impl Edition {
	fn into_book(self, asked_isbn: &str) -> Book {
		let mut ids: Vec<ExternalId> = self
			.identifiers
			.isbn_13
			.iter()
			.chain(self.identifiers.isbn_10.iter())
			.map(ExternalId::isbn)
			.collect();
		if ids.is_empty() {
			ids.push(ExternalId::isbn(asked_isbn));
		}

		Book {
			ids,
			title: self.title,
			subtitle: None,
			authors: self.authors.into_iter().map(|author| author.name).collect(),
			author_asins: Vec::new(),
			narrators: Vec::new(),
			series: Vec::new(),
			description: None,
			cover_url: self.cover.and_then(|cover| cover.large),
			release_date: self.publish_date,
			runtime_minutes: None,
			language: None,
			publisher: self.publishers.into_iter().next().map(|p| p.name),
			genres: self
				.subjects
				.into_iter()
				.take(GENRE_LIMIT)
				.map(|subject| subject.name)
				.collect(),
			tags: Vec::new(),
			format_type: None,
			rating: None,
			is_adult: None,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const EDITION: &str = r#"{
		"title": "The Hobbit",
		"authors": [{"name": "J.R.R. Tolkien"}],
		"identifiers": {"isbn_10": ["0007487290"], "isbn_13": ["9780007487295"]},
		"publishers": [{"name": "HarperCollins"}],
		"publish_date": "Aug 30, 2012",
		"subjects": [{"name": "Fantasy"}, {"name": "Arkenstone"}],
		"cover": {"large": "https://covers.openlibrary.org/b/id/10291546-L.jpg"}
	}"#;

	#[test]
	fn an_edition_carries_every_isbn_it_is_known_by() {
		let book = serde_json::from_str::<Edition>(EDITION)
			.unwrap()
			.into_book("9780007487295");
		assert_eq!(
			book.ids,
			vec![
				ExternalId::isbn("9780007487295"),
				ExternalId::isbn("0007487290"),
			],
			"both forms reach the same work through the id map"
		);
		assert_eq!(book.authors, vec!["J.R.R. Tolkien"]);
		assert_eq!(book.publisher.as_deref(), Some("HarperCollins"));
		assert_eq!(book.genres, vec!["Fantasy", "Arkenstone"]);
		assert_eq!(
			book.cover_url.as_deref(),
			Some("https://covers.openlibrary.org/b/id/10291546-L.jpg")
		);
	}

	#[test]
	fn the_asked_isbn_is_kept_when_the_edition_lists_none() {
		let book = serde_json::from_str::<Edition>(r#"{"title": "Bare"}"#)
			.unwrap()
			.into_book("9781705009055");
		assert_eq!(
			book.ids,
			vec![ExternalId::isbn("9781705009055")],
			"the identifier that found the edition is an identifier it answers to"
		);
	}
}
