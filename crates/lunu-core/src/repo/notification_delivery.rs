use async_trait::async_trait;

use crate::Result;

#[async_trait]
pub trait NotificationDeliveryRepo: Send + Sync {
	async fn delivered_channels(&self, job_id: &str) -> Result<Vec<String>>;
	async fn record(&self, job_id: &str, channel: &str) -> Result<()>;
	async fn clear(&self, job_id: &str) -> Result<()>;
	async fn prune_orphaned(&self) -> Result<u64>;
}
