use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use lunu_core::consts::jobs::{DEFAULT_WORKER_COUNT, LEASE_TIMEOUT_SECS, POLL_INTERVAL_MS};
use lunu_core::models::Job;
use lunu_core::repo::JobRepo;
use lunu_core::traits::JobHandler;

#[derive(Debug, Clone, Copy)]
pub struct WorkerConfig {
	pub workers: usize,
	pub poll_interval: Duration,
	pub lease_timeout: Duration,
}

impl Default for WorkerConfig {
	fn default() -> Self {
		Self {
			workers: DEFAULT_WORKER_COUNT,
			poll_interval: Duration::from_millis(POLL_INTERVAL_MS),
			lease_timeout: Duration::from_secs(LEASE_TIMEOUT_SECS as u64),
		}
	}
}

pub struct WorkerPool {
	jobs: Arc<dyn JobRepo>,
	handler: Arc<dyn JobHandler>,
	config: WorkerConfig,
}

impl WorkerPool {
	pub fn new(jobs: Arc<dyn JobRepo>, handler: Arc<dyn JobHandler>, config: WorkerConfig) -> Self {
		Self {
			jobs,
			handler,
			config,
		}
	}

	pub fn start(self) {
		for index in 0..self.config.workers {
			let jobs = self.jobs.clone();
			let handler = self.handler.clone();
			let poll_interval = self.config.poll_interval;
			let worker_id = format!("worker-{index}");
			tokio::spawn(worker_loop(worker_id, jobs, handler, poll_interval));
		}

		let jobs = self.jobs.clone();
		let lease_timeout = self.config.lease_timeout;
		tokio::spawn(reaper_loop(jobs, lease_timeout));
	}
}

async fn worker_loop(
	worker_id: String,
	jobs: Arc<dyn JobRepo>,
	handler: Arc<dyn JobHandler>,
	poll_interval: Duration,
) {
	loop {
		match jobs.claim_next(&worker_id, Utc::now()).await {
			Ok(Some(job)) => run_job(&jobs, &handler, job).await,
			Ok(None) => tokio::time::sleep(poll_interval).await,
			Err(error) => {
				tracing::error!(%error, worker = %worker_id, "failed to claim job");
				tokio::time::sleep(poll_interval).await;
			}
		}
	}
}

async fn run_job(jobs: &Arc<dyn JobRepo>, handler: &Arc<dyn JobHandler>, job: Job) {
	let outcome = handler.handle(&job).await;
	let now = Utc::now();

	let result = match outcome {
		Ok(()) => jobs.complete(&job.id, now).await,
		Err(error) => {
			let error = error.to_string();
			if job.should_retry() {
				let run_after = now + job.retry_backoff();
				tracing::warn!(job = %job.id, kind = %job.job_type, attempt = job.attempts, %error, "job failed, retrying");
				jobs.reschedule(&job.id, &error, run_after, now).await
			} else {
				tracing::error!(job = %job.id, kind = %job.job_type, %error, "job failed permanently");
				jobs.fail(&job.id, &error, now).await
			}
		}
	};

	if let Err(error) = result {
		tracing::error!(job = %job.id, %error, "failed to record job outcome");
	}
}

async fn reaper_loop(jobs: Arc<dyn JobRepo>, lease_timeout: Duration) {
	let lease = chrono::Duration::seconds(lease_timeout.as_secs() as i64);
	loop {
		tokio::time::sleep(lease_timeout).await;
		let now = Utc::now();
		match jobs.reap_stale(now - lease, now).await {
			Ok(0) => {}
			Ok(reaped) => tracing::warn!(reaped, "reclaimed stale jobs"),
			Err(error) => tracing::error!(%error, "failed to reap stale jobs"),
		}
	}
}

#[cfg(test)]
mod tests {
	use std::sync::Mutex;

	use async_trait::async_trait;
	use chrono::{DateTime, Utc};
	use lunu_core::models::{Job, JobStatus, JobType};
	use lunu_core::{Error, Result};

	use super::*;

	#[derive(Default)]
	struct RecordingRepo {
		action: Mutex<Option<String>>,
	}

	impl RecordingRepo {
		fn action(&self) -> Option<String> {
			self.action.lock().unwrap().clone()
		}
	}

	#[async_trait]
	impl JobRepo for RecordingRepo {
		async fn create(&self, _job: &Job) -> Result<()> {
			Ok(())
		}
		async fn find_by_id(&self, _id: &str) -> Result<Option<Job>> {
			Ok(None)
		}
		async fn list(&self) -> Result<Vec<Job>> {
			Ok(Vec::new())
		}
		async fn claim_next(&self, _worker_id: &str, _now: DateTime<Utc>) -> Result<Option<Job>> {
			Ok(None)
		}
		async fn complete(&self, _id: &str, _at: DateTime<Utc>) -> Result<()> {
			*self.action.lock().unwrap() = Some("complete".to_string());
			Ok(())
		}
		async fn reschedule(
			&self,
			_id: &str,
			_error: &str,
			_run_after: DateTime<Utc>,
			_at: DateTime<Utc>,
		) -> Result<()> {
			*self.action.lock().unwrap() = Some("reschedule".to_string());
			Ok(())
		}
		async fn fail(&self, _id: &str, _error: &str, _at: DateTime<Utc>) -> Result<()> {
			*self.action.lock().unwrap() = Some("fail".to_string());
			Ok(())
		}
		async fn reap_stale(&self, _older_than: DateTime<Utc>, _at: DateTime<Utc>) -> Result<u64> {
			Ok(0)
		}
	}

	struct OkHandler;
	#[async_trait]
	impl JobHandler for OkHandler {
		async fn handle(&self, _job: &Job) -> Result<()> {
			Ok(())
		}
	}

	struct ErrHandler;
	#[async_trait]
	impl JobHandler for ErrHandler {
		async fn handle(&self, _job: &Job) -> Result<()> {
			Err(Error::Integration("boom".to_string()))
		}
	}

	fn job(attempts: i64, max_attempts: i64) -> Job {
		let now = Utc::now();
		Job {
			id: "j1".to_string(),
			job_type: JobType::Search,
			payload: "{}".to_string(),
			status: JobStatus::Running,
			attempts,
			max_attempts,
			run_after: now,
			locked_by: Some("worker-0".to_string()),
			locked_at: Some(now),
			last_error: None,
			created_at: now,
			updated_at: now,
		}
	}

	#[tokio::test]
	async fn success_completes() {
		let repo = Arc::new(RecordingRepo::default());
		let jobs: Arc<dyn JobRepo> = repo.clone();
		let handler: Arc<dyn JobHandler> = Arc::new(OkHandler);
		run_job(&jobs, &handler, job(1, 3)).await;
		assert_eq!(repo.action().as_deref(), Some("complete"));
	}

	#[tokio::test]
	async fn failure_below_max_reschedules() {
		let repo = Arc::new(RecordingRepo::default());
		let jobs: Arc<dyn JobRepo> = repo.clone();
		let handler: Arc<dyn JobHandler> = Arc::new(ErrHandler);
		run_job(&jobs, &handler, job(1, 3)).await;
		assert_eq!(repo.action().as_deref(), Some("reschedule"));
	}

	#[tokio::test]
	async fn failure_at_max_fails() {
		let repo = Arc::new(RecordingRepo::default());
		let jobs: Arc<dyn JobRepo> = repo.clone();
		let handler: Arc<dyn JobHandler> = Arc::new(ErrHandler);
		run_job(&jobs, &handler, job(3, 3)).await;
		assert_eq!(repo.action().as_deref(), Some("fail"));
	}
}
