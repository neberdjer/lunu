use std::sync::Arc;

use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::models::{
	GrabPayload, ImportPayload, Job, JobType, MergePayload, MonitorPayload, NotificationEvent,
};
use lunu_core::services::{
	AuthService, GrabService, ImportService, JobService, LibraryService, MergeService,
	MonitorService, NotificationService, RequestService,
};
use lunu_core::traits::{JobHandler, MergeOutcome};

pub struct PipelineHandler {
	grabs: Arc<GrabService>,
	monitor: Arc<MonitorService>,
	imports: Arc<ImportService>,
	merges: Arc<MergeService>,
	notifications: Arc<NotificationService>,
	requests: Arc<RequestService>,
	library: Arc<LibraryService>,
	auth: Arc<AuthService>,
	jobs: Arc<JobService>,
}

impl PipelineHandler {
	#[allow(clippy::too_many_arguments)]
	pub fn new(
		grabs: Arc<GrabService>,
		monitor: Arc<MonitorService>,
		imports: Arc<ImportService>,
		merges: Arc<MergeService>,
		notifications: Arc<NotificationService>,
		requests: Arc<RequestService>,
		library: Arc<LibraryService>,
		auth: Arc<AuthService>,
		jobs: Arc<JobService>,
	) -> Self {
		Self {
			grabs,
			monitor,
			imports,
			merges,
			notifications,
			requests,
			library,
			auth,
			jobs,
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

	async fn merge(&self, payload: &str) -> Result<()> {
		let payload: MergePayload = serde_json::from_str(payload)?;
		match self.merges.merge(&payload.media_id).await? {
			MergeOutcome::Merged(summary) => tracing::info!(
				media_id = %payload.media_id,
				output = %summary.output_path,
				chapters = summary.chapters,
				sources = summary.handled_sources,
				"merged an audiobook into a single m4b"
			),
			MergeOutcome::Skipped(reason) => tracing::info!(
				media_id = %payload.media_id,
				reason,
				"skipped merging this item"
			),
		}
		Ok(())
	}

	async fn revert_merge(&self, payload: &str) -> Result<()> {
		let payload: MergePayload = serde_json::from_str(payload)?;
		let restored = self.merges.revert(&payload.media_id).await?;
		tracing::info!(
			media_id = %payload.media_id,
			restored,
			"reverted a merge and restored its source files"
		);
		Ok(())
	}

	async fn job_cleanup(&self) -> Result<()> {
		let pruned = self.jobs.prune_finished().await?;
		if pruned > 0 {
			tracing::info!(pruned, "pruned finished jobs past the retention window");
		}
		Ok(())
	}

	async fn library_sync(&self) -> Result<()> {
		let summary = self.library.sync().await?;
		tracing::info!(
			total = summary.total,
			imported = summary.imported,
			updated = summary.updated,
			skipped = summary.skipped,
			matched = summary.matched,
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
			JobType::Merge => self.merge(&job.payload).await,
			JobType::MergeRevert => self.revert_merge(&job.payload).await,
			JobType::Notify => self.notify(&job.payload).await,
			JobType::LibrarySync => self.library_sync().await,
			JobType::SessionCleanup => self.auth.cleanup_expired_sessions().await,
			JobType::JobCleanup => self.job_cleanup().await,
		}
	}

	async fn on_failed(&self, job: &Job, error: &str) {
		if job.job_type.media_subject()
			&& let Ok(payload) = serde_json::from_str::<MergePayload>(&job.payload)
		{
			tracing::error!(job = %job.id, kind = %job.job_type, media_id = %payload.media_id, %error, "merge job exhausted retries");
			if let Err(err) = self.merges.fail(&payload.media_id, error).await {
				tracing::error!(%err, media_id = %payload.media_id, "failed to mark the merge failed");
			}
			return;
		}
		let Some(request_id) = job.request_id.as_deref() else {
			let kind = if job.job_type.is_recurring() {
				"recurring job exhausted retries"
			} else {
				"detached job exhausted retries"
			};
			tracing::error!(job = %job.id, kind = %job.job_type, %error, kind);
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
