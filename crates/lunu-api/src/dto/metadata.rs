use lunu_core::models::{Book, ExternalId, SeriesRef};
use serde::Serialize;

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct SeriesRefResponse {
	pub name: String,
	pub position: Option<String>,
	pub asin: Option<String>,
}

impl From<&SeriesRef> for SeriesRefResponse {
	fn from(series: &SeriesRef) -> Self {
		Self {
			name: series.name.clone(),
			position: series.position.clone(),
			asin: series.asin.clone(),
		}
	}
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct BookResponse {
	pub id: Option<String>,
	pub asin: Option<String>,
	pub title: String,
	pub subtitle: Option<String>,
	pub authors: Vec<String>,
	pub author_asins: Vec<String>,
	pub narrators: Vec<String>,
	pub series: Vec<SeriesRefResponse>,
	pub description: Option<String>,
	pub cover_url: Option<String>,
	pub release_date: Option<String>,
	pub runtime_minutes: Option<i64>,
	pub language: Option<String>,
	pub publisher: Option<String>,
	pub genres: Vec<String>,
	pub tags: Vec<String>,
	pub isbn: Option<String>,
	pub format_type: Option<String>,
	pub rating: Option<String>,
	pub is_adult: Option<bool>,
}

impl From<&Book> for BookResponse {
	fn from(book: &Book) -> Self {
		Self {
			id: book.ids.first().map(ExternalId::to_string),
			asin: book.asin().map(str::to_string),
			title: book.title.clone(),
			subtitle: book.subtitle.clone(),
			authors: book.authors.clone(),
			author_asins: book.author_asins.clone(),
			narrators: book.narrators.clone(),
			series: book.series.iter().map(SeriesRefResponse::from).collect(),
			description: book.description.clone(),
			cover_url: book.cover_url.clone(),
			release_date: book.release_date.clone(),
			runtime_minutes: book.runtime_minutes,
			language: book.language.clone(),
			publisher: book.publisher.clone(),
			genres: book.genres.clone(),
			tags: book.tags.clone(),
			isbn: book.isbn().map(str::to_string),
			format_type: book.format_type.clone(),
			rating: book.rating.clone(),
			is_adult: book.is_adult,
		}
	}
}
