use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Setting {
	pub key: String,
	pub value: String,
	pub encrypted: bool,
	pub updated_at: DateTime<Utc>,
}
