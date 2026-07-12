use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::Result;
use crate::models::UserNotification;

#[async_trait]
pub trait UserNotificationRepo: Send + Sync {
	async fn create(&self, notification: &UserNotification) -> Result<()>;
	async fn list_for_user(
		&self,
		user_id: &str,
		limit: i64,
		offset: i64,
	) -> Result<Vec<UserNotification>>;
	async fn count_for_user(&self, user_id: &str) -> Result<i64>;
	async fn unread_count(&self, user_id: &str) -> Result<i64>;
	async fn mark_read(&self, user_id: &str, id: &str, at: DateTime<Utc>) -> Result<bool>;
	async fn mark_all_read(&self, user_id: &str, at: DateTime<Utc>) -> Result<u64>;
}
