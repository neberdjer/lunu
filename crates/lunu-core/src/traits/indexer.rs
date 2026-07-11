use async_trait::async_trait;

use crate::Result;
use crate::models::Release;

#[async_trait]
pub trait Indexer: Send + Sync {
	fn id(&self) -> &'static str;
	async fn search(&self, query: &str) -> Result<Vec<Release>>;
	async fn test_connection(&self) -> Result<()>;
}
