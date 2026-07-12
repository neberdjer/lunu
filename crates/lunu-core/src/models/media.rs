use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Media {
	pub asin: String,
	pub title: String,
	pub author: Option<String>,
	pub cover_url: Option<String>,
	pub library_path: String,
	pub request_id: Option<String>,
	pub created_at: DateTime<Utc>,
}
