use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct BlocklistEntry {
	pub id: String,
	pub request_id: String,
	pub download_url: String,
	pub created_at: DateTime<Utc>,
}
