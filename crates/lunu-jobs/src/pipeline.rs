use std::sync::Arc;

use async_trait::async_trait;
use lunu_core::models::{
	GrabPayload, ImportPayload, Job, JobType, MonitorPayload, NotificationEvent,
};
use lunu_core::services::{GrabService, ImportService, MonitorService, NotificationService};
use lunu_core::traits::JobHandler;
use lunu_core::{Error, Result};

pub struct PipelineHandler {
	grabs: Arc<GrabService>,
	monitor: Arc<MonitorService>,
	imports: Arc<ImportService>,
	notifications: Arc<NotificationService>,
}

impl PipelineHandler {
	pub fn new(
		grabs: Arc<GrabService>,
		monitor: Arc<MonitorService>,
		imports: Arc<ImportService>,
		notifications: Arc<NotificationService>,
	) -> Self {
		Self {
			grabs,
			monitor,
			imports,
			notifications,
		}
	}

	async fn notify(&self, payload: &str) -> Result<()> {
		let event: NotificationEvent = serde_json::from_str(payload)?;
		self.notifications.dispatch(&event).await
	}

	async fn grab(&self, payload: &str) -> Result<()> {
		let payload: GrabPayload = serde_json::from_str(payload)?;
		self.grabs.grab(&payload.request_id, None).await?;
		Ok(())
	}

	async fn monitor(&self, payload: &str) -> Result<()> {
		let payload: MonitorPayload = serde_json::from_str(payload)?;
		self.monitor
			.poll(&payload.download_id, payload.misses)
			.await
	}

	async fn import(&self, payload: &str) -> Result<()> {
		let payload: ImportPayload = serde_json::from_str(payload)?;
		self.imports
			.import(&payload.download_id, &payload.content_path)
			.await
	}
}

#[async_trait]
impl JobHandler for PipelineHandler {
	async fn handle(&self, job: &Job) -> Result<()> {
		match job.job_type {
			JobType::Grab => self.grab(&job.payload).await,
			JobType::MonitorDownload => self.monitor(&job.payload).await,
			JobType::Import => self.import(&job.payload).await,
			JobType::Notify => self.notify(&job.payload).await,
			other => Err(Error::Internal(format!(
				"job stage not yet implemented: {other}"
			))),
		}
	}
}
