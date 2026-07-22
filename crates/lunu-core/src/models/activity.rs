use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy)]
pub enum ActivityTarget<'a> {
	Request(&'a str),
	Media(&'a str),
}

#[derive(Debug, Clone)]
pub struct Activity {
	pub id: String,
	pub request_id: Option<String>,
	pub media_id: Option<String>,
	pub event: String,
	pub detail: Option<String>,
	pub actor: Option<String>,
	pub at: DateTime<Utc>,
}
