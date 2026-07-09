use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct ApiKey {
	pub id: String,
	pub user_id: String,
	pub name: String,
	pub prefix: String,
	pub key_hash: String,
	pub scopes: Vec<String>,
	pub created_at: DateTime<Utc>,
	pub last_used_at: Option<DateTime<Utc>>,
	pub expires_at: Option<DateTime<Utc>>,
	pub revoked: bool,
}

impl ApiKey {
	pub fn is_active(&self, now: DateTime<Utc>) -> bool {
		!self.revoked && self.expires_at.is_none_or(|expires| expires > now)
	}
}
