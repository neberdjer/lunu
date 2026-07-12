use async_trait::async_trait;

use crate::Result;
use crate::models::BlocklistEntry;

#[async_trait]
pub trait BlocklistRepo: Send + Sync {
	async fn add(&self, entry: &BlocklistEntry) -> Result<()>;
	async fn urls_for_request(&self, request_id: &str) -> Result<Vec<String>>;
	async fn list_for_request(&self, request_id: &str) -> Result<Vec<BlocklistEntry>>;
	async fn remove(&self, request_id: &str, download_url: &str) -> Result<bool>;
}
