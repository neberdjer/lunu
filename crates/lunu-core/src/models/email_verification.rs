use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct EmailVerificationToken {
	pub id: String,
	pub user_id: String,
	pub code_hash: String,
	pub attempts: i64,
	pub created_at: DateTime<Utc>,
	pub expires_at: DateTime<Utc>,
}

impl EmailVerificationToken {
	pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
		self.expires_at <= now
	}
}
