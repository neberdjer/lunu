use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::Result;
use crate::models::Job;

#[async_trait]
pub trait JobRepo: Send + Sync {
	async fn create(&self, job: &Job) -> Result<()>;
	async fn find_by_id(&self, id: &str) -> Result<Option<Job>>;
	async fn list(&self) -> Result<Vec<Job>>;
	async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<Job>>;
	async fn count(&self) -> Result<i64>;
	async fn claim_next(&self, worker_id: &str, now: DateTime<Utc>) -> Result<Option<Job>>;
	async fn complete(&self, id: &str, at: DateTime<Utc>) -> Result<()>;
	async fn reschedule(
		&self,
		id: &str,
		error: &str,
		run_after: DateTime<Utc>,
		at: DateTime<Utc>,
	) -> Result<()>;
	async fn fail(&self, id: &str, error: &str, at: DateTime<Utc>) -> Result<()>;
	async fn reap_stale(&self, older_than: DateTime<Utc>, at: DateTime<Utc>) -> Result<u64>;
}
