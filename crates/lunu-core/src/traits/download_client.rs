use async_trait::async_trait;

use crate::Result;

#[async_trait]
pub trait DownloadClient: Send + Sync {
	fn id(&self) -> &'static str;
	async fn add(&self, download_url: &str, category: &str) -> Result<()>;
}
