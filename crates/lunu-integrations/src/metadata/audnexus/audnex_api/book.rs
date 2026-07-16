use lunu_core::models::{Book, SeriesRef};
use serde::Deserialize;

use super::super::text::normalize_date;
use super::super::{Named, asins, names};

const TAG_KIND: &str = "tag";

#[derive(Deserialize)]
struct AudnexusSeries {
	name: String,
	position: Option<String>,
	asin: Option<String>,
}

#[derive(Deserialize, Clone)]
struct AudnexusGenre {
	name: String,
	#[serde(rename = "type", default)]
	kind: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AudnexusAuthor {
	pub(super) name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AudnexusBook {
	asin: String,
	title: String,
	subtitle: Option<String>,
	#[serde(default)]
	authors: Vec<Named>,
	#[serde(default)]
	narrators: Vec<Named>,
	series_primary: Option<AudnexusSeries>,
	series_secondary: Option<AudnexusSeries>,
	description: Option<String>,
	image: Option<String>,
	release_date: Option<String>,
	runtime_length_min: Option<i64>,
	language: Option<String>,
	publisher_name: Option<String>,
	#[serde(default)]
	genres: Vec<AudnexusGenre>,
	isbn: Option<String>,
	format_type: Option<String>,
	rating: Option<String>,
	is_adult: Option<bool>,
}

impl AudnexusBook {
	pub(super) fn into_book(self) -> Book {
		let series = [self.series_primary, self.series_secondary]
			.into_iter()
			.flatten()
			.map(|entry| SeriesRef {
				name: entry.name,
				position: entry.position,
				asin: entry.asin,
			})
			.collect();

		let (genres, tags) = split_genres(&self.genres);

		Book {
			asin: self.asin,
			title: self.title,
			subtitle: self.subtitle,
			authors: names(&self.authors),
			author_asins: asins(&self.authors),
			narrators: names(&self.narrators),
			series,
			description: self.description,
			cover_url: self.image,
			release_date: normalize_date(self.release_date),
			runtime_minutes: self.runtime_length_min,
			language: self.language,
			publisher: self.publisher_name,
			genres,
			tags,
			isbn: self.isbn,
			format_type: self.format_type,
			rating: self.rating,
			is_adult: self.is_adult,
		}
	}
}

fn split_genres(entries: &[AudnexusGenre]) -> (Vec<String>, Vec<String>) {
	let (tags, genres): (Vec<_>, Vec<_>) = entries
		.iter()
		.partition(|entry| entry.kind.as_deref() == Some(TAG_KIND));

	let name = |entry: &&AudnexusGenre| entry.name.clone();
	(
		genres.iter().map(name).collect(),
		tags.iter().map(name).collect(),
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	const AUDNEXUS_BOOK: &str = r#"{
		"asin": "1705009050",
		"title": "The Hobbit",
		"subtitle": "or There and Back Again",
		"authors": [{"asin": "A1", "name": "J.R.R. Tolkien"}],
		"narrators": [{"name": "Rob Inglis"}],
		"seriesPrimary": {"asin": "S1", "name": "The Lord of the Rings", "position": "0"},
		"description": "A great adventure.",
		"image": "https://images/cover.jpg",
		"releaseDate": "2012-01-01T00:00:00.000Z",
		"runtimeLengthMin": 662,
		"language": "english",
		"publisherName": "Recorded Books",
		"genres": [
			{"asin": "G1", "name": "Fantasy", "type": "genre"},
			{"asin": "G2", "name": "Literature & Fiction", "type": "genre"},
			{"asin": "T1", "name": "Epic", "type": "tag"}
		],
		"isbn": "9780007487295",
		"formatType": "unabridged",
		"rating": "4.9",
		"isAdult": false
	}"#;

	fn hobbit() -> Book {
		serde_json::from_str::<AudnexusBook>(AUDNEXUS_BOOK)
			.unwrap()
			.into_book()
	}

	#[test]
	fn parses_audnexus_book() {
		let book = hobbit();
		assert_eq!(book.asin, "1705009050");
		assert_eq!(book.authors, vec!["J.R.R. Tolkien"]);
		assert_eq!(book.narrators, vec!["Rob Inglis"]);
		assert_eq!(book.series.len(), 1);
		assert_eq!(book.series[0].name, "The Lord of the Rings");
		assert_eq!(book.series[0].position.as_deref(), Some("0"));
		assert_eq!(book.runtime_minutes, Some(662));
		assert_eq!(book.cover_url.as_deref(), Some("https://images/cover.jpg"));
	}

	#[test]
	fn genres_and_tags_are_kept_apart() {
		let book = hobbit();
		assert_eq!(book.genres, vec!["Fantasy", "Literature & Fiction"]);
		assert_eq!(
			book.tags,
			vec!["Epic"],
			"audnexus marks tags with type=tag, and folding them into genres inflates every book"
		);
	}

	#[test]
	fn captures_identity_and_edition_fields() {
		let book = hobbit();
		assert_eq!(
			book.isbn.as_deref(),
			Some("9780007487295"),
			"isbn is the only identifier a non-audible provider could match on"
		);
		assert_eq!(book.format_type.as_deref(), Some("unabridged"));
		assert_eq!(book.rating.as_deref(), Some("4.9"));
		assert_eq!(book.is_adult, Some(false));
	}

	#[test]
	fn release_dates_are_normalized_to_match_audible() {
		assert_eq!(
			hobbit().release_date.as_deref(),
			Some("2012-01-01"),
			"audnexus sends a timestamp where audible sends a date"
		);
	}
}
