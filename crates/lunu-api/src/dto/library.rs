use lunu_core::models::Media;
use serde::Serialize;

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct MediaResponse {
	pub id: String,
	pub asin: Option<String>,
	pub title: String,
	pub author: Option<String>,
	pub cover_url: Option<String>,
	pub series_name: Option<String>,
	pub series_sequence: Option<String>,
	pub source: String,
	pub overridden: bool,
}

impl From<&Media> for MediaResponse {
	fn from(media: &Media) -> Self {
		Self {
			id: media.id.clone(),
			asin: media.asin.clone(),
			title: media.title.clone(),
			author: media.author.clone(),
			cover_url: media.cover_url.clone(),
			series_name: media.series_name.clone(),
			series_sequence: media.series_sequence.clone(),
			source: media.source.as_str().to_string(),
			overridden: media.overridden,
		}
	}
}
