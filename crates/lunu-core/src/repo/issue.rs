use async_trait::async_trait;

use crate::Result;
use crate::models::Issue;

#[async_trait]
pub trait IssueRepo: Send + Sync {
	async fn create(&self, issue: &Issue) -> Result<()>;
	async fn find_by_id(&self, id: &str) -> Result<Option<Issue>>;
	async fn list_page(&self, status: Option<&str>, limit: i64, offset: i64) -> Result<Vec<Issue>>;
	async fn count(&self, status: Option<&str>) -> Result<i64>;
	async fn for_request(&self, request_id: &str) -> Result<Vec<Issue>>;
	async fn update(&self, issue: &Issue) -> Result<()>;
}
