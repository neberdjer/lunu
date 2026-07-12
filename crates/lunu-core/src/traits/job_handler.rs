use async_trait::async_trait;

use crate::Result;
use crate::models::Job;

#[async_trait]
pub trait JobHandler: Send + Sync {
	async fn handle(&self, job: &Job) -> Result<()>;

	async fn on_failed(&self, _job: &Job, _error: &str) {}
}
