use chrono::{DateTime, Utc};
use lunu_core::models::Watch;
use serde::Serialize;

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct WatchResponse {
	pub id: String,
	pub user_id: String,
	pub asin: Option<String>,
	pub title: String,
	pub author: Option<String>,
	pub cover_url: Option<String>,
	pub series_name: Option<String>,
	pub series_sequence: Option<String>,
	pub created_at: DateTime<Utc>,
}

impl From<&Watch> for WatchResponse {
	fn from(watch: &Watch) -> Self {
		Self {
			id: watch.id.clone(),
			user_id: watch.user_id.clone(),
			asin: watch.asin.clone(),
			title: watch.title.clone(),
			author: watch.author.clone(),
			cover_url: watch.cover_url.clone(),
			series_name: watch.series_name.clone(),
			series_sequence: watch.series_sequence.clone(),
			created_at: watch.created_at,
		}
	}
}
