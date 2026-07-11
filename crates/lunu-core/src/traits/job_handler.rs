use async_trait::async_trait;

use crate::Result;
use crate::models::Job;

#[async_trait]
pub trait JobHandler: Send + Sync {
	async fn handle(&self, job: &Job) -> Result<()>;
}
