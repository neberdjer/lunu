use lunu_core::consts::metadata::METADATA_GENRE_LIMIT as GENRE_LIMIT;
use lunu_core::models::{Book, ExternalId};
use serde::Deserialize;

#[derive(Deserialize)]
pub(super) struct VolumeResponse {
	#[serde(default)]
	items: Vec<Volume>,
}

impl VolumeResponse {
	pub(super) fn into_books(self) -> Vec<Book> {
		self.items
			.into_iter()
			.filter_map(Volume::into_book)
			.collect()
	}
}

#[derive(Deserialize)]
struct Volume {
	#[serde(rename = "volumeInfo")]
	volume_info: VolumeInfo,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VolumeInfo {
	title: Option<String>,
	subtitle: Option<String>,
	#[serde(default)]
	authors: Vec<String>,
	publisher: Option<String>,
	published_date: Option<String>,
	description: Option<String>,
	#[serde(default)]
	industry_identifiers: Vec<IndustryIdentifier>,
	#[serde(default)]
	categories: Vec<String>,
	language: Option<String>,
	maturity_rating: Option<String>,
	image_links: Option<ImageLinks>,
}

#[derive(Deserialize)]
struct IndustryIdentifier {
	#[serde(rename = "type")]
	kind: String,
	identifier: String,
}

#[derive(Deserialize)]
struct ImageLinks {
	thumbnail: Option<String>,
	small_thumbnail: Option<String>,
}

impl Volume {
	fn into_book(self) -> Option<Book> {
		let info = self.volume_info;
		let title = info.title.filter(|title| !title.is_empty())?;
		let ids = isbn_ids(&info.industry_identifiers);
		let cover_url = info
			.image_links
			.and_then(|links| links.thumbnail.or(links.small_thumbnail))
			.map(crate::to_https);

		Some(Book {
			ids,
			title,
			subtitle: info.subtitle,
			authors: info.authors,
			author_asins: Vec::new(),
			narrators: Vec::new(),
			series: Vec::new(),
			description: info.description,
			cover_url,
			release_date: info.published_date,
			runtime_minutes: None,
			language: info.language,
			publisher: info.publisher,
			genres: info.categories.into_iter().take(GENRE_LIMIT).collect(),
			tags: Vec::new(),
			format_type: None,
			rating: None,
			is_adult: info
				.maturity_rating
				.map(|rating| rating.eq_ignore_ascii_case("MATURE")),
		})
	}
}

fn isbn_ids(identifiers: &[IndustryIdentifier]) -> Vec<ExternalId> {
	let of_kind = |kind| identifiers.iter().filter(move |id| id.kind == kind);
	of_kind("ISBN_13")
		.chain(of_kind("ISBN_10"))
		.map(|id| ExternalId::isbn(&id.identifier))
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	const VOLUMES: &str = r#"{
		"items": [{
			"volumeInfo": {
				"title": "The Hobbit",
				"subtitle": "Or There and Back Again",
				"authors": ["J.R.R. Tolkien"],
				"publisher": "HarperCollins",
				"publishedDate": "2012-08-30",
				"description": "Bilbo Baggins enjoys a comfortable life.",
				"industryIdentifiers": [
					{"type": "ISBN_10", "identifier": "0007487290"},
					{"type": "ISBN_13", "identifier": "9780007487295"},
					{"type": "OTHER", "identifier": "OCLC:1234"}
				],
				"categories": ["Fiction", "Fantasy"],
				"language": "en",
				"maturityRating": "NOT_MATURE",
				"imageLinks": {"thumbnail": "http://books.google.com/cover.jpg"}
			}
		}, {
			"volumeInfo": {"title": "Bare Volume"}
		}]
	}"#;

	fn books() -> Vec<Book> {
		serde_json::from_str::<VolumeResponse>(VOLUMES)
			.unwrap()
			.into_books()
	}

	#[test]
	fn a_volume_maps_isbn13_first_and_upgrades_the_cover_to_https() {
		let book = &books()[0];
		assert_eq!(
			book.ids,
			vec![
				ExternalId::isbn("9780007487295"),
				ExternalId::isbn("0007487290"),
			],
			"isbn 13 leads, isbn 10 follows, non-isbn identifiers are dropped"
		);
		assert_eq!(book.title, "The Hobbit");
		assert_eq!(book.subtitle.as_deref(), Some("Or There and Back Again"));
		assert_eq!(book.authors, vec!["J.R.R. Tolkien"]);
		assert!(book.description.as_deref().unwrap().contains("Bilbo"));
		assert_eq!(book.genres, vec!["Fiction", "Fantasy"]);
		assert_eq!(book.is_adult, Some(false));
		assert_eq!(
			book.cover_url.as_deref(),
			Some("https://books.google.com/cover.jpg"),
			"google serves http thumbnails, which must be upgraded before storage"
		);
	}

	#[test]
	fn a_volume_without_identifiers_keeps_its_title_but_no_ids() {
		let book = &books()[1];
		assert!(
			book.ids.is_empty(),
			"a volume this source cannot identify must not invent an isbn"
		);
		assert_eq!(book.title, "Bare Volume");
	}

	#[test]
	fn a_titleless_volume_is_dropped_rather_than_failing_the_batch() {
		let response: VolumeResponse = serde_json::from_str(
			r#"{"items": [{"volumeInfo": {"authors": ["Nobody"]}}, {"volumeInfo": {"title": "Real"}}]}"#,
		)
		.unwrap();
		let books = response.into_books();
		assert_eq!(
			books.len(),
			1,
			"a volume the api omitted a title for must not abort the whole page"
		);
		assert_eq!(books[0].title, "Real");
	}
}
