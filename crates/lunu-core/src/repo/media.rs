use async_trait::async_trait;

use crate::Result;
use crate::models::Media;

#[async_trait]
pub trait MediaRepo: Send + Sync {
	async fn upsert(&self, media: &Media) -> Result<()>;
	async fn find_by_asin(&self, asin: &str) -> Result<Option<Media>>;
	async fn available_among(&self, asins: &[String]) -> Result<Vec<String>>;
	async fn count(&self) -> Result<i64>;
}
