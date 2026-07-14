use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::Result;
use crate::models::Schedule;

#[async_trait]
pub trait ScheduleRepo: Send + Sync {
	async fn insert_if_absent(&self, schedule: &Schedule) -> Result<()>;
	async fn list(&self) -> Result<Vec<Schedule>>;
	async fn due(&self, now: DateTime<Utc>) -> Result<Vec<Schedule>>;
	async fn advance(
		&self,
		kind: &str,
		last_run: DateTime<Utc>,
		next_run: DateTime<Utc>,
	) -> Result<()>;
	async fn configure(
		&self,
		kind: &str,
		enabled: bool,
		interval_secs: i64,
		next_run: DateTime<Utc>,
	) -> Result<bool>;
}
