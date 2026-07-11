use async_trait::async_trait;

use crate::Result;
use crate::models::QualityProfile;

#[async_trait]
pub trait QualityProfileRepo: Send + Sync {
	async fn create(&self, profile: &QualityProfile) -> Result<()>;
	async fn update(&self, profile: &QualityProfile) -> Result<()>;
	async fn find_by_id(&self, id: &str) -> Result<Option<QualityProfile>>;
	async fn find_default(&self) -> Result<Option<QualityProfile>>;
	async fn list(&self) -> Result<Vec<QualityProfile>>;
	async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<QualityProfile>>;
	async fn count(&self) -> Result<i64>;
	async fn set_default(&self, id: &str) -> Result<()>;
	async fn delete(&self, id: &str) -> Result<()>;
}
