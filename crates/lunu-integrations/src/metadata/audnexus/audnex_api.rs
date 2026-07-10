use lunu_core::Result;
use lunu_core::models::{Book, Chapter, Chapters, SeriesRef};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use super::{Named, integration_error, names};

const AUDNEXUS_BASE: &str = "https://api.audnex.us";

pub(super) async fn get_book(
	client: &reqwest::Client,
	region: &str,
	asin: &str,
) -> Result<Option<Book>> {
	Ok(
		get_json::<AudnexusBook>(client, region, &format!("books/{asin}"))
			.await?
			.map(AudnexusBook::into_book),
	)
}

pub(super) async fn get_chapters(
	client: &reqwest::Client,
	region: &str,
	asin: &str,
) -> Result<Option<Chapters>> {
	Ok(
		get_json::<AudnexusChapters>(client, region, &format!("books/{asin}/chapters"))
			.await?
			.map(AudnexusChapters::into_chapters),
	)
}

async fn get_json<T: DeserializeOwned>(
	client: &reqwest::Client,
	region: &str,
	path: &str,
) -> Result<Option<T>> {
	let response = client
		.get(format!("{AUDNEXUS_BASE}/{path}"))
		.query(&[("region", region)])
		.send()
		.await
		.map_err(integration_error)?;

	if response.status() == reqwest::StatusCode::NOT_FOUND {
		return Ok(None);
	}

	let response = response.error_for_status().map_err(integration_error)?;
	Ok(Some(response.json::<T>().await.map_err(integration_error)?))
}

#[derive(Deserialize)]
struct AudnexusSeries {
	name: String,
	position: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AudnexusBook {
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
	genres: Vec<Named>,
}

impl AudnexusBook {
	fn into_book(self) -> Book {
		let series = [self.series_primary, self.series_secondary]
			.into_iter()
			.flatten()
			.map(|entry| SeriesRef {
				name: entry.name,
				position: entry.position,
			})
			.collect();

		Book {
			asin: self.asin,
			title: self.title,
			subtitle: self.subtitle,
			authors: names(self.authors),
			narrators: names(self.narrators),
			series,
			description: self.description,
			cover_url: self.image,
			release_date: self.release_date,
			runtime_minutes: self.runtime_length_min,
			language: self.language,
			publisher: self.publisher_name,
			genres: names(self.genres),
		}
	}
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AudnexusChapters {
	asin: String,
	runtime_length_ms: Option<i64>,
	#[serde(default)]
	chapters: Vec<AudnexusChapter>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AudnexusChapter {
	title: String,
	start_offset_ms: i64,
	length_ms: i64,
}

impl AudnexusChapters {
	fn into_chapters(self) -> Chapters {
		Chapters {
			asin: self.asin,
			runtime_ms: self.runtime_length_ms,
			chapters: self
				.chapters
				.into_iter()
				.map(|chapter| Chapter {
					title: chapter.title,
					start_offset_ms: chapter.start_offset_ms,
					length_ms: chapter.length_ms,
				})
				.collect(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const AUDNEXUS_BOOK: &str = r#"{
		"asin": "B0036I54I6",
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
		"genres": [{"asin": "G1", "name": "Fantasy"}]
	}"#;

	const AUDNEXUS_CHAPTERS: &str = r#"{
		"asin": "B0036I54I6",
		"runtimeLengthMs": 39724444,
		"chapters": [
			{"title": "Opening Credits", "startOffsetMs": 0, "lengthMs": 12000},
			{"title": "Chapter 1", "startOffsetMs": 12000, "lengthMs": 1200000}
		]
	}"#;

	#[test]
	fn parses_audnexus_book() {
		let book: AudnexusBook = serde_json::from_str(AUDNEXUS_BOOK).unwrap();
		let book = book.into_book();
		assert_eq!(book.asin, "B0036I54I6");
		assert_eq!(book.authors, vec!["J.R.R. Tolkien"]);
		assert_eq!(book.narrators, vec!["Rob Inglis"]);
		assert_eq!(book.series.len(), 1);
		assert_eq!(book.series[0].name, "The Lord of the Rings");
		assert_eq!(book.series[0].position.as_deref(), Some("0"));
		assert_eq!(book.runtime_minutes, Some(662));
		assert_eq!(book.cover_url.as_deref(), Some("https://images/cover.jpg"));
		assert_eq!(book.genres, vec!["Fantasy"]);
	}

	#[test]
	fn parses_audnexus_chapters() {
		let chapters: AudnexusChapters = serde_json::from_str(AUDNEXUS_CHAPTERS).unwrap();
		let chapters = chapters.into_chapters();
		assert_eq!(chapters.runtime_ms, Some(39724444));
		assert_eq!(chapters.chapters.len(), 2);
		assert_eq!(chapters.chapters[1].start_offset_ms, 12000);
		assert_eq!(chapters.chapters[1].title, "Chapter 1");
	}
}
