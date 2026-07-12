use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Activity {
	pub id: String,
	pub request_id: String,
	pub event: String,
	pub detail: Option<String>,
	pub actor: Option<String>,
	pub at: DateTime<Utc>,
}
