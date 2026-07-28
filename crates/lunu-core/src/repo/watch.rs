use async_trait::async_trait;

use crate::Result;
use crate::models::Watch;

#[async_trait]
pub trait WatchRepo: Send + Sync {
	async fn create(&self, watch: &Watch) -> Result<()>;
	async fn find_for_user(&self, user_id: &str, id: &str) -> Result<Option<Watch>>;
	async fn list_page(&self, user_id: &str, limit: i64, offset: i64) -> Result<Vec<Watch>>;
	async fn count(&self, user_id: &str) -> Result<i64>;
	async fn delete_owned(&self, user_id: &str, id: &str) -> Result<bool>;
}
