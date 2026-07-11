use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::Result;
use crate::models::Request;

#[async_trait]
pub trait RequestRepo: Send + Sync {
	async fn create(&self, request: &Request) -> Result<()>;
	async fn update(&self, request: &Request) -> Result<()>;
	async fn find_by_id(&self, id: &str) -> Result<Option<Request>>;
	async fn find_by_user_and_asin(&self, user_id: &str, asin: &str) -> Result<Option<Request>>;
	async fn list(&self) -> Result<Vec<Request>>;
	async fn list_for_user(&self, user_id: &str) -> Result<Vec<Request>>;
	async fn list_page(
		&self,
		user_id: Option<&str>,
		status: Option<&str>,
		limit: i64,
		offset: i64,
	) -> Result<Vec<Request>>;
	async fn count(&self, user_id: Option<&str>, status: Option<&str>) -> Result<i64>;
	async fn count_for_user_since(&self, user_id: &str, since: DateTime<Utc>) -> Result<i64>;
	async fn delete(&self, id: &str) -> Result<()>;
}
