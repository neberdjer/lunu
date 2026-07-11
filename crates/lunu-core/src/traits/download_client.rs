use async_trait::async_trait;

use crate::Result;
use crate::models::DownloadStatus;

#[async_trait]
pub trait DownloadClient: Send + Sync {
	fn id(&self) -> &'static str;
	async fn add(&self, download_url: &str, category: &str) -> Result<()>;
	async fn status(&self, info_hash: &str) -> Result<Option<DownloadStatus>>;
	async fn test_connection(&self) -> Result<()>;
}
