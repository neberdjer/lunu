use async_trait::async_trait;

use crate::Result;
use crate::models::{DownloadStatus, Protocol};

#[async_trait]
pub trait DownloadClient: Send + Sync {
	fn id(&self) -> &'static str;
	fn protocol(&self) -> Protocol;
	async fn is_configured(&self) -> Result<bool>;
	async fn add(&self, download_url: &str, category: &str) -> Result<Option<String>>;
	async fn status(&self, client_ref: &str) -> Result<Option<DownloadStatus>>;
	async fn remove(&self, client_ref: &str, delete_files: bool) -> Result<()>;
	async fn test_connection(&self) -> Result<()>;
}
