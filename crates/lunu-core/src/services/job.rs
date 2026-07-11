use std::sync::Arc;

use chrono::Utc;

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

	pub async fn enqueue(&self, job_type: JobType, payload: String) -> Result<Job> {
		let now = Utc::now();
		let job = Job {
			id: new_id(),
			job_type,
			payload,
			status: JobStatus::Pending,
			attempts: 0,
			max_attempts: DEFAULT_MAX_ATTEMPTS,
			run_after: now,
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
