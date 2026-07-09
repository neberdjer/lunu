use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Session {
	pub id: String,
	pub user_id: String,
	pub token_hash: String,
	pub created_at: DateTime<Utc>,
	pub expires_at: DateTime<Utc>,
	pub last_seen_at: Option<DateTime<Utc>>,
	pub user_agent: Option<String>,
}

impl Session {
	pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
		self.expires_at <= now
	}
}
