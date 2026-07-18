use lunu_core::models::{Book, ExternalId};
use serde::Deserialize;

use super::{GENRE_LIMIT, NARRATOR_ROLE, format_rating};
use crate::{nonempty, to_https};

#[derive(Deserialize)]
pub(super) struct EditionsData {
	#[serde(default)]
	editions: Vec<Edition>,
}

impl EditionsData {
	pub(super) fn into_book(self) -> Option<Book> {
		self.editions
			.into_iter()
			.next()
			.and_then(Edition::into_book)
	}
}

#[derive(Deserialize)]
struct Edition {
	isbn_13: Option<String>,
	isbn_10: Option<String>,
	asin: Option<String>,
	audio_seconds: Option<i64>,
	release_date: Option<String>,
	language: Option<Language>,
	publisher: Option<Publisher>,
	#[serde(default)]
	contributions: Vec<Contribution>,
	book: Option<EditionBook>,
}

#[derive(Deserialize)]
struct Language {
	language: Option<String>,
}

#[derive(Deserialize)]
struct Publisher {
	name: Option<String>,
}

#[derive(Deserialize)]
struct Contribution {
	contribution: Option<String>,
	author: Option<Author>,
}

#[derive(Deserialize)]
struct Author {
	name: Option<String>,
}

#[derive(Deserialize)]
struct EditionBook {
	title: Option<String>,
	subtitle: Option<String>,
	description: Option<String>,
	release_date: Option<String>,
	rating: Option<f64>,
	#[serde(default)]
	cached_tags: Vec<CachedTag>,
	image: Option<Image>,
}

#[derive(Deserialize)]
struct CachedTag {
	tag: Option<String>,
}

#[derive(Deserialize)]
struct Image {
	url: Option<String>,
}

impl Edition {
	fn into_book(self) -> Option<Book> {
		let book = self.book?;
		let title = nonempty(book.title)?;
		let ids = [self.isbn_13, self.isbn_10]
			.into_iter()
			.flatten()
			.map(ExternalId::isbn)
			.chain(
				self.asin
					.filter(|asin| !asin.is_empty())
					.map(ExternalId::asin),
			)
			.collect();
		let (authors, narrators) = split_contributors(self.contributions);

		Some(Book {
			ids,
			title,
			subtitle: nonempty(book.subtitle),
			authors,
			author_asins: Vec::new(),
			narrators,
			series: Vec::new(),
			description: nonempty(book.description),
			cover_url: book.image.and_then(|image| image.url).map(to_https),
			release_date: self.release_date.or(book.release_date),
			runtime_minutes: self
				.audio_seconds
				.filter(|seconds| *seconds > 0)
				.map(|seconds| seconds / 60),
			language: nonempty(self.language.and_then(|language| language.language)),
			publisher: self.publisher.and_then(|publisher| publisher.name),
			genres: book
				.cached_tags
				.into_iter()
				.filter_map(|tag| tag.tag)
				.take(GENRE_LIMIT)
				.collect(),
			tags: Vec::new(),
			format_type: None,
			rating: format_rating(book.rating),
			is_adult: None,
		})
	}
}

fn split_contributors(contributions: Vec<Contribution>) -> (Vec<String>, Vec<String>) {
	let mut authors = Vec::new();
	let mut narrators = Vec::new();
	for contribution in contributions {
		let Some(name) = contribution.author.and_then(|author| author.name) else {
			continue;
		};
		match contribution.contribution {
			None => authors.push(name),
			Some(role) if role.eq_ignore_ascii_case(NARRATOR_ROLE) => narrators.push(name),
			Some(_) => {}
		}
	}
	(authors, narrators)
}

#[cfg(test)]
mod tests {
	use super::*;

	const EDITION: &str = r#"{
		"editions": [{
			"isbn_13": "9781705009055",
			"isbn_10": "1705009050",
			"asin": "1705009050",
			"audio_seconds": 37500,
			"release_date": "2019-11-05",
			"language": {"language": "English"},
			"publisher": {"name": "Recorded Books, Inc."},
			"contributions": [
				{"contribution": null, "author": {"name": "J.R.R. Tolkien"}},
				{"contribution": "Narrator", "author": {"name": "Andy Serkis"}}
			],
			"book": {
				"title": "The Hobbit",
				"subtitle": "There and Back Again",
				"description": "In a hole in the ground there lived a hobbit.",
				"release_date": "1937-01-01",
				"rating": 4.314015,
				"cached_tags": [
					{"tag": "Fantasy"},
					{"tag": "Classics"}
				],
				"image": {"url": "http://covers.hardcover.app/hobbit.jpg"}
			}
		}]
	}"#;

	fn book() -> Book {
		serde_json::from_str::<EditionsData>(EDITION)
			.unwrap()
			.into_book()
			.unwrap()
	}

	#[test]
	fn an_edition_maps_isbns_before_the_asin_and_upgrades_the_cover_to_https() {
		let book = book();
		assert_eq!(
			book.ids,
			vec![
				ExternalId::isbn("9781705009055"),
				ExternalId::isbn("1705009050"),
				ExternalId::asin("1705009050"),
			],
			"isbns lead so results route back through the isbn-keyed provider, asin follows as a cross-link"
		);
		assert_eq!(
			book.cover_url.as_deref(),
			Some("https://covers.hardcover.app/hobbit.jpg")
		);
	}

	#[test]
	fn the_null_role_is_the_author_and_the_narrator_role_is_the_narrator() {
		let book = book();
		assert_eq!(book.authors, vec!["J.R.R. Tolkien"]);
		assert_eq!(book.narrators, vec!["Andy Serkis"]);
	}

	#[test]
	fn audio_seconds_become_whole_runtime_minutes() {
		assert_eq!(book().runtime_minutes, Some(625));
	}

	#[test]
	fn genres_and_edition_facts_come_through() {
		let book = book();
		assert_eq!(book.genres, vec!["Fantasy", "Classics"]);
		assert_eq!(book.language.as_deref(), Some("English"));
		assert_eq!(book.publisher.as_deref(), Some("Recorded Books, Inc."));
		assert_eq!(book.rating.as_deref(), Some("4.31"));
		assert_eq!(book.release_date.as_deref(), Some("2019-11-05"));
	}

	#[test]
	fn an_edition_without_a_linked_book_is_dropped() {
		let data: EditionsData =
			serde_json::from_str(r#"{"editions": [{"isbn_13": "x"}]}"#).unwrap();
		assert!(data.into_book().is_none());
	}
}
