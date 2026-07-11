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
	async fn clear_default(&self) -> Result<()>;
	async fn delete(&self, id: &str) -> Result<()>;
}
