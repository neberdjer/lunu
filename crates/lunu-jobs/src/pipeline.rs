use std::sync::Arc;

use async_trait::async_trait;
use lunu_core::models::{GrabPayload, Job, JobType};
use lunu_core::services::GrabService;
use lunu_core::traits::JobHandler;
use lunu_core::{Error, Result};

pub struct PipelineHandler {
	grabs: Arc<GrabService>,
}

impl PipelineHandler {
	pub fn new(grabs: Arc<GrabService>) -> Self {
		Self { grabs }
	}

	async fn grab(&self, payload: &str) -> Result<()> {
		let payload: GrabPayload = serde_json::from_str(payload)?;
		self.grabs.grab(&payload.request_id, None).await?;
		Ok(())
	}
}

#[async_trait]
impl JobHandler for PipelineHandler {
	async fn handle(&self, job: &Job) -> Result<()> {
		match job.job_type {
			JobType::Grab => self.grab(&job.payload).await,
			other => Err(Error::Internal(format!(
				"job stage not yet implemented: {other}"
			))),
		}
	}
}
