use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::consts::jobs::DEFAULT_MAX_ATTEMPTS;
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
		let now = Utc::now();
		let job = Job {
			id: new_id(),
			job_type,
			request_id: Some(request_id.to_string()),
			payload: serde_json::to_string(payload)?,
			status: JobStatus::Pending,
			attempts: 0,
			max_attempts: DEFAULT_MAX_ATTEMPTS,
			run_after,
			locked_by: None,
			locked_at: None,
			last_error: None,
			created_at: now,
			updated_at: now,
		};
		self.jobs.create(&job).await?;
		Ok(job)
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

	pub async fn get(&self, id: &str) -> Result<Option<Job>> {
		self.jobs.find_by_id(id).await
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
