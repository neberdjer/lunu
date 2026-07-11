use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::Result;
use crate::consts::jobs::DEFAULT_MAX_ATTEMPTS;
use crate::models::{Job, JobStatus, JobType};
use crate::repo::JobRepo;
use crate::services::new_id;

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

	pub async fn enqueue<T: Serialize + ?Sized>(
		&self,
		job_type: JobType,
		payload: &T,
	) -> Result<Job> {
		self.enqueue_at(job_type, payload, Utc::now()).await
	}

	pub async fn enqueue_at<T: Serialize + ?Sized>(
		&self,
		job_type: JobType,
		payload: &T,
		run_after: DateTime<Utc>,
	) -> Result<Job> {
		let now = Utc::now();
		let job = Job {
			id: new_id(),
			job_type,
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

	pub async fn get(&self, id: &str) -> Result<Option<Job>> {
		self.jobs.find_by_id(id).await
	}
}
