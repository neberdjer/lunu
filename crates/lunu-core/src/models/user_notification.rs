use chrono::{DateTime, Utc};

use crate::models::NotificationKind;

#[derive(Debug, Clone)]
pub struct UserNotification {
	pub id: String,
	pub user_id: String,
	pub kind: NotificationKind,
	pub request_id: Option<String>,
	pub title: String,
	pub created_at: DateTime<Utc>,
	pub read_at: Option<DateTime<Utc>>,
}
