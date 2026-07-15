use std::sync::Arc;

use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::models::{
	GrabPayload, ImportPayload, Job, JobType, MonitorPayload, NotificationEvent,
};
use lunu_core::services::{
	AuthService, GrabService, ImportService, LibraryService, MonitorService, NotificationService,
	RequestService,
};
use lunu_core::traits::JobHandler;

pub struct PipelineHandler {
	grabs: Arc<GrabService>,
	monitor: Arc<MonitorService>,
	imports: Arc<ImportService>,
	notifications: Arc<NotificationService>,
	requests: Arc<RequestService>,
	library: Arc<LibraryService>,
	auth: Arc<AuthService>,
}

impl PipelineHandler {
	#[allow(clippy::too_many_arguments)]
	pub fn new(
		grabs: Arc<GrabService>,
		monitor: Arc<MonitorService>,
		imports: Arc<ImportService>,
		notifications: Arc<NotificationService>,
		requests: Arc<RequestService>,
		library: Arc<LibraryService>,
		auth: Arc<AuthService>,
	) -> Self {
		Self {
			grabs,
			monitor,
			imports,
			notifications,
			requests,
			library,
			auth,
		}
	}

	async fn notify(&self, payload: &str) -> Result<()> {
		let event: NotificationEvent = serde_json::from_str(payload)?;
		let report = self.notifications.dispatch(&event).await?;
		if report.failed > 0 {
			tracing::warn!(
				delivered = report.delivered,
				failed = report.failed,
				kind = %event.kind,
				"notification dispatch had failures"
			);
		}
		if report.total_failure()
			&& let Some(error) = report.last_error
		{
			return Err(error);
		}
		Ok(())
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

	async fn library_sync(&self) -> Result<()> {
		let summary = self.library.sync().await?;
		tracing::info!(
			total = summary.total,
			imported = summary.imported,
			updated = summary.updated,
			skipped = summary.skipped,
			"audiobookshelf library sync complete"
		);
		Ok(())
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
			JobType::LibrarySync => self.library_sync().await,
			JobType::SessionCleanup => self.auth.cleanup_expired_sessions().await,
		}
	}

	async fn on_failed(&self, job: &Job, error: &str) {
		let Some(request_id) = job.request_id.as_deref() else {
			tracing::error!(job = %job.id, kind = %job.job_type, %error, "recurring job exhausted retries");
			return;
		};
		if !job.job_type.propagates_failure_to_request() {
			return;
		}
		tracing::error!(job = %job.id, kind = %job.job_type, request = %request_id, %error, "request fulfillment failed permanently");
		if let Err(err) = self.requests.mark_failed(request_id, None).await {
			tracing::error!(%err, request = %request_id, "failed to mark request failed after job exhaustion");
		}
	}
}
