use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::Result;
use crate::models::ApiKey;

#[async_trait]
pub trait ApiKeyRepo: Send + Sync {
	async fn create(&self, key: &ApiKey) -> Result<()>;
	async fn find_by_key_hash(&self, key_hash: &str) -> Result<Option<ApiKey>>;
	async fn list_for_user_page(
		&self,
		user_id: &str,
		limit: i64,
		offset: i64,
	) -> Result<Vec<ApiKey>>;
	async fn count_for_user(&self, user_id: &str) -> Result<i64>;
	async fn touch_last_used(&self, id: &str, at: DateTime<Utc>) -> Result<()>;
	async fn revoke_owned(&self, id: &str, user_id: &str) -> Result<bool>;
}
