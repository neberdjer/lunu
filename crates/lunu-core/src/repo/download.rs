use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::Result;
use crate::models::{Download, DownloadState};

#[async_trait]
pub trait DownloadRepo: Send + Sync {
	async fn create(&self, download: &Download) -> Result<()>;
	async fn find_by_id(&self, id: &str) -> Result<Option<Download>>;
	async fn find_by_request(&self, request_id: &str) -> Result<Option<Download>>;
	async fn list(&self) -> Result<Vec<Download>>;
	async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<Download>>;
	async fn count(&self) -> Result<i64>;
	async fn update_status(
		&self,
		id: &str,
		state: DownloadState,
		progress: i64,
		at: DateTime<Utc>,
	) -> Result<()>;
	async fn delete(&self, id: &str) -> Result<()>;
}
