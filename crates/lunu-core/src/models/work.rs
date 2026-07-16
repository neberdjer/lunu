use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Work {
	pub id: String,
	pub title: String,
	pub author: Option<String>,
	pub cover_url: Option<String>,
	pub created_at: DateTime<Utc>,
}
