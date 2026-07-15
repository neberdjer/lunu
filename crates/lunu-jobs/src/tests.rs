use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use lunu_core::consts::jobs::TRANSIENT_MAX_ATTEMPTS;
use lunu_core::models::{Job, JobStatus, JobType};
use lunu_core::repo::JobRepo;
use lunu_core::traits::JobHandler;
use lunu_core::{Error, Result};

use crate::run_job;

#[derive(Default)]
struct RecordingRepo {
	action: Mutex<Option<String>>,
	rescheduled_max_attempts: Mutex<Option<i64>>,
}

impl RecordingRepo {
	fn action(&self) -> Option<String> {
		self.action.lock().unwrap().clone()
	}

	fn rescheduled_max_attempts(&self) -> Option<i64> {
		*self.rescheduled_max_attempts.lock().unwrap()
	}
}

#[async_trait]
impl JobRepo for RecordingRepo {
	async fn create(&self, _job: &Job) -> Result<()> {
		Ok(())
	}
	async fn create_recurring(&self, _job: &Job) -> Result<bool> {
		Ok(true)
	}
	async fn find_by_id(&self, _id: &str) -> Result<Option<Job>> {
		Ok(None)
	}
	async fn list(&self) -> Result<Vec<Job>> {
		Ok(Vec::new())
	}
	async fn list_page(
		&self,
		_status: Option<&str>,
		_limit: i64,
		_offset: i64,
	) -> Result<Vec<Job>> {
		Ok(Vec::new())
	}
	async fn count(&self, _status: Option<&str>) -> Result<i64> {
		Ok(0)
	}
	async fn requeue(&self, _id: &str, _at: DateTime<Utc>) -> Result<bool> {
		Ok(true)
	}
	async fn delete(&self, _id: &str) -> Result<bool> {
		Ok(true)
	}
	async fn has_active(&self, _job_type: &str, _request_id: &str) -> Result<bool> {
		Ok(false)
	}
	async fn claim_next(&self, _worker_id: &str, _now: DateTime<Utc>) -> Result<Option<Job>> {
		Ok(None)
	}
	async fn renew_lease(&self, _id: &str, _locked_by: &str, _now: DateTime<Utc>) -> Result<bool> {
		Ok(true)
	}
	async fn complete(&self, _id: &str, _locked_by: &str, _at: DateTime<Utc>) -> Result<()> {
		*self.action.lock().unwrap() = Some("complete".to_string());
		Ok(())
	}
	async fn reschedule(
		&self,
		_id: &str,
		_locked_by: &str,
		_error: &str,
		_run_after: DateTime<Utc>,
		_at: DateTime<Utc>,
		max_attempts: i64,
	) -> Result<()> {
		*self.action.lock().unwrap() = Some("reschedule".to_string());
		*self.rescheduled_max_attempts.lock().unwrap() = Some(max_attempts);
		Ok(())
	}
	async fn fail(
		&self,
		_id: &str,
		_locked_by: &str,
		_error: &str,
		_at: DateTime<Utc>,
	) -> Result<()> {
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

struct PermanentErrHandler;
#[async_trait]
impl JobHandler for PermanentErrHandler {
	async fn handle(&self, _job: &Job) -> Result<()> {
		Err(Error::Validation("bad".to_string()))
	}
}

struct TimeoutErrHandler;
#[async_trait]
impl JobHandler for TimeoutErrHandler {
	async fn handle(&self, _job: &Job) -> Result<()> {
		Err(Error::Internal("job exceeded maximum duration".to_string()))
	}
}

fn job(attempts: i64, max_attempts: i64) -> Job {
	let now = Utc::now();
	Job {
		id: "j1".to_string(),
		job_type: JobType::Grab,
		request_id: None,
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
async fn permanent_failure_below_max_reschedules() {
	let repo = Arc::new(RecordingRepo::default());
	let jobs: Arc<dyn JobRepo> = repo.clone();
	let handler: Arc<dyn JobHandler> = Arc::new(PermanentErrHandler);
	run_job(&jobs, &handler, job(1, 3)).await;
	assert_eq!(repo.action().as_deref(), Some("reschedule"));
	assert_eq!(
		repo.rescheduled_max_attempts(),
		Some(3),
		"a permanent error keeps the job's own retry budget"
	);
}

#[tokio::test]
async fn permanent_failure_at_max_fails() {
	let repo = Arc::new(RecordingRepo::default());
	let jobs: Arc<dyn JobRepo> = repo.clone();
	let handler: Arc<dyn JobHandler> = Arc::new(PermanentErrHandler);
	run_job(&jobs, &handler, job(3, 3)).await;
	assert_eq!(repo.action().as_deref(), Some("fail"));
}

#[tokio::test]
async fn transient_failure_at_max_reschedules_and_raises_the_budget() {
	let repo = Arc::new(RecordingRepo::default());
	let jobs: Arc<dyn JobRepo> = repo.clone();
	let handler: Arc<dyn JobHandler> = Arc::new(ErrHandler);
	run_job(&jobs, &handler, job(3, 3)).await;
	assert_eq!(repo.action().as_deref(), Some("reschedule"));
	assert_eq!(
		repo.rescheduled_max_attempts(),
		Some(TRANSIENT_MAX_ATTEMPTS),
		"a transient error raises the persisted budget so should_retry stays authoritative"
	);
}

#[tokio::test]
async fn timeout_is_not_transient_and_fails_at_max() {
	let repo = Arc::new(RecordingRepo::default());
	let jobs: Arc<dyn JobRepo> = repo.clone();
	let handler: Arc<dyn JobHandler> = Arc::new(TimeoutErrHandler);
	run_job(&jobs, &handler, job(3, 3)).await;
	assert_eq!(
		repo.action().as_deref(),
		Some("fail"),
		"a job that exceeded its maximum duration must not re-arm the transient budget"
	);
}
