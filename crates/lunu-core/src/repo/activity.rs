use async_trait::async_trait;

use crate::Result;
use crate::models::Activity;

#[async_trait]
pub trait ActivityRepo: Send + Sync {
	async fn create(&self, activity: &Activity) -> Result<()>;
	async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<Activity>>;
	async fn count(&self) -> Result<i64>;
	async fn for_request(&self, request_id: &str) -> Result<Vec<Activity>>;
}
