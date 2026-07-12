use std::sync::Arc;

use async_trait::async_trait;
use lunu_core::models::{
	GrabPayload, ImportPayload, Job, JobType, MonitorPayload, NotificationEvent,
};
use lunu_core::services::{
	GrabService, ImportService, MonitorService, NotificationService, RequestService,
};
use lunu_core::traits::JobHandler;
use lunu_core::{Error, Result};

pub struct PipelineHandler {
	grabs: Arc<GrabService>,
	monitor: Arc<MonitorService>,
	imports: Arc<ImportService>,
	notifications: Arc<NotificationService>,
	requests: Arc<RequestService>,
}

impl PipelineHandler {
	pub fn new(
		grabs: Arc<GrabService>,
		monitor: Arc<MonitorService>,
		imports: Arc<ImportService>,
		notifications: Arc<NotificationService>,
		requests: Arc<RequestService>,
	) -> Self {
		Self {
			grabs,
			monitor,
			imports,
			notifications,
			requests,
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
		self.monitor.poll(&payload).await
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

	async fn on_failed(&self, job: &Job, error: &str) {
		let Some(request_id) = job.request_id.as_deref() else {
			return;
		};
		if !job.job_type.propagates_failure_to_request() {
			return;
		}
		if let Err(err) = self.requests.mark_failed(request_id, Some(error)).await {
			tracing::error!(%err, request = %request_id, "failed to mark request failed after job exhaustion");
		}
	}
}
