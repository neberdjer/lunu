use chrono::{DateTime, Utc};

use crate::models::Role;

#[derive(Debug, Clone)]
pub struct Invite {
	pub id: String,
	pub code_hash: String,
	pub role: Role,
	pub email: Option<String>,
	pub created_by: String,
	pub max_uses: i64,
	pub used_count: i64,
	pub created_at: DateTime<Utc>,
	pub expires_at: Option<DateTime<Utc>>,
}

impl Invite {
	pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
		self.expires_at.is_some_and(|expires| expires <= now)
	}
}
