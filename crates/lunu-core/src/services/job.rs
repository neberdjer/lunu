use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;

use crate::consts::jobs::{DEFAULT_MAX_ATTEMPTS, JOB_RETENTION_DAYS};
use crate::consts::reasons;
use crate::models::{Job, JobStatus, JobType};
use crate::repo::JobRepo;
use crate::services::new_id;
use crate::{Error, Result};

pub struct JobService {
	jobs: Arc<dyn JobRepo>,
}

impl JobService {
	pub fn new(jobs: Arc<dyn JobRepo>) -> Self {
		Self { jobs }
	}

	pub fn repo(&self) -> Arc<dyn JobRepo> {
		self.jobs.clone()
	}

	pub async fn has_active(&self, job_type: JobType, request_id: &str) -> Result<bool> {
		self.jobs.has_active(job_type.as_str(), request_id).await
	}

	pub async fn prune_finished(&self) -> Result<u64> {
		let cutoff = Utc::now() - Duration::days(JOB_RETENTION_DAYS);
		self.jobs.delete_finished_before(cutoff).await
	}

	pub async fn enqueue_for<T: Serialize + ?Sized>(
		&self,
		job_type: JobType,
		payload: &T,
		request_id: &str,
	) -> Result<Job> {
		self.enqueue_for_at(job_type, payload, request_id, Utc::now())
			.await
	}

	pub async fn enqueue_for_at<T: Serialize + ?Sized>(
		&self,
		job_type: JobType,
		payload: &T,
		request_id: &str,
		run_after: DateTime<Utc>,
	) -> Result<Job> {
		self.enqueue(job_type, payload, Some(request_id), run_after)
			.await
	}

	async fn enqueue<T: Serialize + ?Sized>(
		&self,
		job_type: JobType,
		payload: &T,
		request_id: Option<&str>,
		run_after: DateTime<Utc>,
	) -> Result<Job> {
		let job = build_job(
			job_type,
			request_id.map(str::to_string),
			serde_json::to_string(payload)?,
			run_after,
		);
		self.jobs.create(&job).await?;
		Ok(job)
	}

	pub async fn enqueue_detached(&self, job_type: JobType) -> Result<bool> {
		let job = build_job(job_type, None, "null".to_string(), Utc::now());
		self.jobs.create_recurring(&job).await
	}

	pub async fn enqueue_unique_with<T: Serialize + ?Sized>(
		&self,
		job_type: JobType,
		payload: &T,
		dedupe_key: &str,
	) -> Result<Job> {
		let mut job = build_job(job_type, None, serde_json::to_string(payload)?, Utc::now());
		job.dedupe_key = Some(dedupe_key.to_string());
		if self.jobs.create_recurring(&job).await? {
			return Ok(job);
		}
		match self.jobs.find_active_by_dedupe(dedupe_key).await? {
			Some(existing) => Ok(existing),
			None => {
				self.jobs.create(&job).await?;
				Ok(job)
			}
		}
	}

	pub async fn enqueue_detached_with<T: Serialize + ?Sized>(
		&self,
		job_type: JobType,
		payload: &T,
	) -> Result<Job> {
		self.enqueue(job_type, payload, None, Utc::now()).await
	}

	pub async fn find(&self, id: &str) -> Result<Job> {
		self.jobs
			.find_by_id(id)
			.await?
			.ok_or_else(|| Error::NotFound(format!("job {id}")))
	}

	pub async fn list(&self) -> Result<Vec<Job>> {
		self.jobs.list().await
	}

	pub async fn list_page(
		&self,
		status: Option<&str>,
		limit: i64,
		offset: i64,
	) -> Result<Vec<Job>> {
		self.jobs.list_page(status, limit, offset).await
	}

	pub async fn count(&self, status: Option<&str>) -> Result<i64> {
		self.jobs.count(status).await
	}

	pub async fn requeue(&self, id: &str) -> Result<()> {
		if self.jobs.requeue(id, Utc::now()).await? {
			return Ok(());
		}
		if self.jobs.find_by_id(id).await?.is_none() {
			return Err(Error::NotFound(format!("job {id}")));
		}
		Err(Error::Conflict(reasons::JOB_NOT_RETRYABLE.to_string()))
	}

	pub async fn cancel(&self, id: &str) -> Result<()> {
		if !self.jobs.delete(id).await? {
			return Err(Error::NotFound(format!("job {id}")));
		}
		Ok(())
	}
}

fn build_job(
	job_type: JobType,
	request_id: Option<String>,
	payload: String,
	run_after: DateTime<Utc>,
) -> Job {
	let now = Utc::now();
	Job {
		id: new_id(),
		job_type,
		request_id,
		dedupe_key: None,
		payload,
		status: JobStatus::Pending,
		attempts: 0,
		max_attempts: DEFAULT_MAX_ATTEMPTS,
		run_after,
		locked_by: None,
		locked_at: None,
		last_error: None,
		created_at: now,
		updated_at: now,
	}
}
