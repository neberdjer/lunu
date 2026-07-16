use std::collections::HashMap;

use lunu_core::models::{Book, SeriesRef};
use serde::Deserialize;

use super::super::text::{normalize_date, strip_html};
use super::super::{Named, asins, ids, names};

const GENRES_ROOT: &str = "Genres";

#[derive(Deserialize)]
struct AudibleSeries {
	title: String,
	sequence: Option<String>,
	asin: Option<String>,
}

#[derive(Deserialize)]
struct CategoryLadder {
	#[serde(default)]
	root: Option<String>,
	#[serde(default)]
	ladder: Vec<Named>,
}

#[derive(Deserialize)]
pub(super) struct AudibleProduct {
	pub(super) asin: String,
	title: String,
	subtitle: Option<String>,
	#[serde(default)]
	authors: Vec<Named>,
	#[serde(default)]
	narrators: Vec<Named>,
	#[serde(default)]
	series: Vec<AudibleSeries>,
	#[serde(default)]
	category_ladders: Vec<CategoryLadder>,
	publisher_summary: Option<String>,
	merchandising_summary: Option<String>,
	product_images: Option<HashMap<String, String>>,
	release_date: Option<String>,
	runtime_length_min: Option<i64>,
	language: Option<String>,
	publisher_name: Option<String>,
	format_type: Option<String>,
	isbn: Option<String>,
}

impl AudibleProduct {
	pub(super) fn into_book(self) -> Book {
		let (genres, tags) = split_categories(&self.category_ladders);
		Book {
			ids: ids(self.asin, self.isbn),
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
			description: strip_html(self.publisher_summary.or(self.merchandising_summary)),
			cover_url: largest_image(self.product_images),
			release_date: normalize_date(self.release_date),
			runtime_minutes: self.runtime_length_min,
			language: self.language,
			publisher: self.publisher_name,
			genres,
			tags,
			format_type: self.format_type,
			rating: None,
			is_adult: None,
		}
	}
}

fn split_categories(ladders: &[CategoryLadder]) -> (Vec<String>, Vec<String>) {
	let mut genres = Vec::new();
	let mut tags = Vec::new();

	for ladder in ladders {
		if ladder.root.as_deref() != Some(GENRES_ROOT) {
			continue;
		}
		let mut rungs = ladder.ladder.iter().map(|rung| rung.name.clone());
		if let Some(root) = rungs.next() {
			push_unique(&mut genres, root);
		}
		for rung in rungs {
			push_unique(&mut tags, rung);
		}
	}

	tags.retain(|tag| !genres.contains(tag));
	(genres, tags)
}

fn push_unique(values: &mut Vec<String>, value: String) {
	if !values.contains(&value) {
		values.push(value);
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

	const AUDIBLE_PRODUCT: &str = r#"{
		"asin": "1705009050",
		"title": "The Hobbit",
		"authors": [{"name": "J.R.R. Tolkien"}],
		"narrators": [{"name": "Rob Inglis"}],
		"series": [{"title": "The Lord of the Rings", "sequence": "0", "asin": "B009CFOEGK"}],
		"category_ladders": [
			{"root": "Genres", "ladder": [{"name": "Literature & Fiction"}, {"name": "Action & Adventure"}]},
			{"root": "Genres", "ladder": [{"name": "Literature & Fiction"}, {"name": "Classics"}]},
			{"root": "Contributors", "ladder": [{"name": "Rob Inglis"}]}
		],
		"merchandising_summary": "<p>Truncated blurb...</p>",
		"publisher_summary": "<p>Bilbo Baggins</p><p>lives in a hole.</p>",
		"product_images": {"500": "https://images/500.jpg", "2400": "https://images/2400.jpg"},
		"release_date": "2012-01-01",
		"runtime_length_min": 662,
		"language": "english",
		"publisher_name": "Recorded Books",
		"format_type": "unabridged",
		"isbn": "9780007487295"
	}"#;

	fn hobbit() -> Book {
		serde_json::from_str::<AudibleProduct>(AUDIBLE_PRODUCT)
			.unwrap()
			.into_book()
	}

	#[test]
	fn prefers_the_full_publisher_summary_over_the_truncated_blurb() {
		assert_eq!(
			hobbit().description.as_deref(),
			Some("Bilbo Baggins lives in a hole."),
			"merchandising_summary is cut off mid-sentence, and neither may reach a client as html"
		);
	}

	#[test]
	fn falls_back_to_the_blurb_when_there_is_no_publisher_summary() {
		let mut value: serde_json::Value = serde_json::from_str(AUDIBLE_PRODUCT).unwrap();
		value["publisher_summary"] = serde_json::Value::Null;
		let book = serde_json::from_value::<AudibleProduct>(value)
			.unwrap()
			.into_book();
		assert_eq!(book.description.as_deref(), Some("Truncated blurb..."));
	}

	#[test]
	fn genres_come_from_the_genres_ladder_only() {
		let book = hobbit();
		assert_eq!(
			book.genres,
			vec!["Literature & Fiction"],
			"the ladder root is the genre, and non-genre ladders must not leak in"
		);
		assert_eq!(book.tags, vec!["Action & Adventure", "Classics"]);
	}

	#[test]
	fn the_largest_cover_wins() {
		assert_eq!(
			hobbit().cover_url.as_deref(),
			Some("https://images/2400.jpg")
		);
	}

	#[test]
	fn captures_identity_and_edition_fields() {
		let book = hobbit();
		assert_eq!(book.isbn(), Some("9780007487295"));
		assert_eq!(book.format_type.as_deref(), Some("unabridged"));
		assert_eq!(book.series[0].asin.as_deref(), Some("B009CFOEGK"));
	}
}
