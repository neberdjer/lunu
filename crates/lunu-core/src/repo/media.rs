use async_trait::async_trait;

use crate::Result;
use crate::models::Media;

#[async_trait]
pub trait MediaRepo: Send + Sync {
	async fn upsert_request(&self, media: &Media) -> Result<()>;
	async fn insert(&self, media: &Media) -> Result<()>;
	async fn update(&self, media: &Media) -> Result<()>;
	async fn find_by_asin(&self, asin: &str) -> Result<Option<Media>>;
	async fn find_by_abs_item_id(&self, abs_item_id: &str) -> Result<Option<Media>>;
	async fn find_by_id(&self, id: &str) -> Result<Option<Media>>;
	async fn available_among(&self, asins: &[String]) -> Result<Vec<String>>;
	async fn list_page(&self, unmatched_only: bool, limit: i64, offset: i64) -> Result<Vec<Media>>;
	async fn list_count(&self, unmatched_only: bool) -> Result<i64>;
	async fn count(&self) -> Result<i64>;
}
