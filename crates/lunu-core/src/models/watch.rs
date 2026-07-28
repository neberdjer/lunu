use chrono::{DateTime, Utc};

use crate::models::Format;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Watch {
	pub id: String,
	pub user_id: String,
	pub work_id: String,
	pub format: Format,
	pub asin: Option<String>,
	pub title: String,
	pub author: Option<String>,
	pub cover_url: Option<String>,
	pub series_name: Option<String>,
	pub series_sequence: Option<String>,
	pub created_at: DateTime<Utc>,
}
