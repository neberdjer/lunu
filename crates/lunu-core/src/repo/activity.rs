use async_trait::async_trait;

use crate::Result;
use crate::models::Activity;

#[async_trait]
pub trait ActivityRepo: Send + Sync {
	async fn create(&self, activity: &Activity) -> Result<()>;
	async fn recent(&self, limit: i64) -> Result<Vec<Activity>>;
	async fn for_request(&self, request_id: &str) -> Result<Vec<Activity>>;
}
