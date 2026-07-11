use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct UserSettings {
	pub user_id: String,
	pub auto_approve: bool,
	pub request_quota: Option<i64>,
	pub quota_days: Option<i64>,
	pub updated_at: DateTime<Utc>,
}

impl UserSettings {
	pub fn default_for(user_id: &str) -> Self {
		Self {
			user_id: user_id.to_string(),
			auto_approve: false,
			request_quota: None,
			quota_days: None,
			updated_at: Utc::now(),
		}
	}
}
