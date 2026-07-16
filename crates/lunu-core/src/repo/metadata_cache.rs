use async_trait::async_trait;

use crate::Result;
use crate::models::MetadataCacheEntry;

#[async_trait]
pub trait MetadataCacheRepo: Send + Sync {
	async fn get(
		&self,
		provider: &str,
		kind: &str,
		key: &str,
	) -> Result<Option<MetadataCacheEntry>>;
	async fn put(&self, entry: &MetadataCacheEntry) -> Result<()>;
	async fn delete(&self, kind: &str, key: &str) -> Result<()>;
}
