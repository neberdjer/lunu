use std::sync::Arc;

use crate::consts::merge::{
	ACTIVITY_MERGE_REVERTED, ACTIVITY_MERGE_SKIPPED, ACTIVITY_MERGED, DEFAULT_MERGE_BITRATE,
	MERGE_ALL_LIMIT, MERGE_SKIP_NO_LIBRARY_PATH, SETTING_MERGE_BACKUP_DIR, SETTING_MERGE_BITRATE,
	SETTING_MERGE_ENABLED, SETTING_MERGE_SOURCE_ACTION,
};
use crate::consts::reasons;
use crate::helpers::naming;
use crate::models::{
	ActivityTarget, JobType, LiveEvent, Media, MediaFilter, MergePayload, MergeState,
	SourceDisposition,
};
use crate::repo::MediaRepo;
use crate::services::{ActivityService, JobService, SettingsService};
use crate::traits::{EventPublisher, MergeOutcome, MergePlan, MergePreview, Merger, RevertPlan};
use crate::{Error, Result};

pub struct QueuedMerges {
	pub queued: usize,
	pub truncated: bool,
}

pub struct MergeService {
	media: Arc<dyn MediaRepo>,
	settings: Arc<SettingsService>,
	merger: Arc<dyn Merger>,
	jobs: Arc<JobService>,
	activity: Arc<ActivityService>,
	events: Arc<dyn EventPublisher>,
}

impl MergeService {
	pub fn new(
		media: Arc<dyn MediaRepo>,
		settings: Arc<SettingsService>,
		merger: Arc<dyn Merger>,
		jobs: Arc<JobService>,
		activity: Arc<ActivityService>,
		events: Arc<dyn EventPublisher>,
	) -> Self {
		Self {
			media,
			settings,
			merger,
			jobs,
			activity,
			events,
		}
	}

	pub async fn test(&self) -> Result<()> {
		self.merger.test_connection().await
	}

	pub async fn try_request(&self, media_id: &str) {
		if self
			.settings
			.toggle(SETTING_MERGE_ENABLED)
			.await
			.unwrap_or(false)
		{
			let _ = self.request(media_id).await;
		}
	}

	pub async fn request_all(&self) -> Result<QueuedMerges> {
		self.test().await?;
		let mut candidates = self
			.media
			.list_page(MediaFilter::Mergeable, MERGE_ALL_LIMIT + 1, 0)
			.await?;
		let truncated = candidates.len() > MERGE_ALL_LIMIT as usize;
		candidates.truncate(MERGE_ALL_LIMIT as usize);
		let queued = candidates.len();

		for media in candidates {
			self.enqueue(JobType::Merge, &media.id).await?;
			self.mark(media, MergeState::Queued, None).await?;
		}
		Ok(QueuedMerges { queued, truncated })
	}

	async fn enqueue(&self, job_type: JobType, media_id: &str) -> Result<String> {
		let payload = MergePayload {
			media_id: media_id.to_string(),
		};
		Ok(self
			.jobs
			.enqueue_unique_with(job_type, &payload, &format!("{job_type}:{media_id}"))
			.await?
			.id)
	}

	async fn find(&self, media_id: &str) -> Result<Media> {
		self.media
			.find_by_id(media_id)
			.await?
			.ok_or_else(|| Error::NotFound(format!("media {media_id}")))
	}

	async fn mark(&self, media: Media, state: MergeState, detail: Option<&str>) -> Result<()> {
		self.media.set_merge_state(&media.id, state, detail).await?;
		self.announce(Media {
			merge_state: state,
			merge_detail: detail.map(str::to_string),
			..media
		});
		Ok(())
	}

	async fn save(&self, media: Media) -> Result<()> {
		self.media.update(&media).await?;
		self.announce(media);
		Ok(())
	}

	fn announce(&self, media: Media) {
		self.events.publish(&LiveEvent::Merge(Box::new(media)));
	}

	pub async fn request(&self, media_id: &str) -> Result<String> {
		let media = self.find(media_id).await?;
		self.test().await?;
		let job = self.enqueue(JobType::Merge, media_id).await?;
		self.mark(media, MergeState::Queued, None).await?;
		Ok(job)
	}

	pub async fn request_revert(&self, media_id: &str) -> Result<String> {
		let media = self.find(media_id).await?;
		revert_plan(&media)?;
		self.test().await?;
		self.enqueue(JobType::MergeRevert, media_id).await
	}

	pub async fn revert(&self, media_id: &str) -> Result<usize> {
		let media = self.find(media_id).await?;
		let plan = revert_plan(&media)?;
		let restored = self.merger.revert(&plan).await?;

		self.save(Media {
			merged_path: None,
			merge_backup_path: None,
			merge_state: MergeState::Idle,
			merge_detail: None,
			..media
		})
		.await?;
		self.activity
			.record(
				ActivityTarget::Media(media_id),
				ACTIVITY_MERGE_REVERTED,
				Some(&restored.to_string()),
				None,
			)
			.await?;
		Ok(restored)
	}

	pub async fn fail(&self, media_id: &str, error: &str) -> Result<()> {
		let media = self.find(media_id).await?;
		self.mark(media, MergeState::Failed, Some(error)).await
	}

	async fn plan_for(&self, media: &Media) -> Result<MergePlan> {
		let settings = self
			.settings
			.resolve_many(&[
				SETTING_MERGE_SOURCE_ACTION,
				SETTING_MERGE_BACKUP_DIR,
				SETTING_MERGE_BITRATE,
			])
			.await?;
		let sources = SourceDisposition::resolve(
			settings
				.get(SETTING_MERGE_SOURCE_ACTION)
				.map(String::as_str)
				.unwrap_or_default(),
			settings.get(SETTING_MERGE_BACKUP_DIR).cloned(),
		)?;

		Ok(MergePlan {
			output_path: naming::merged_file(&media.library_path, &media.title),
			source_dir: media.library_path.clone(),
			title: media.title.clone(),
			author: media.author.clone(),
			series: media.series_name.clone(),
			sequence: media.series_sequence.clone(),
			bitrate: settings
				.get(SETTING_MERGE_BITRATE)
				.cloned()
				.unwrap_or_else(|| DEFAULT_MERGE_BITRATE.to_string()),
			sources,
			previous_output: media.merged_path.clone(),
		})
	}

	pub async fn preview(&self, media_id: &str) -> Result<MergePreview> {
		let media = self.find(media_id).await?;
		if media.library_path.trim().is_empty() {
			return Ok(MergePreview {
				output_path: String::new(),
				skip: Some(MERGE_SKIP_NO_LIBRARY_PATH),
				chapters: Vec::new(),
				total_seconds: 0.0,
				copyable: false,
				sources: SourceDisposition::Keep,
				bitrate: String::new(),
			});
		}
		self.test().await?;
		let plan = self.plan_for(&media).await?;
		self.merger.preview(&plan).await
	}

	pub async fn merge(&self, media_id: &str) -> Result<MergeOutcome> {
		let media = self.find(media_id).await?;
		if media.library_path.trim().is_empty() {
			return self
				.settle(media, MergeOutcome::Skipped(MERGE_SKIP_NO_LIBRARY_PATH))
				.await;
		}
		let plan = self.plan_for(&media).await?;
		let outcome = self.merger.merge(&plan).await?;
		self.settle(media, outcome).await
	}

	async fn settle(&self, media: Media, outcome: MergeOutcome) -> Result<MergeOutcome> {
		let id = media.id.clone();
		let (event, detail) = match &outcome {
			MergeOutcome::Merged(summary) => {
				self.save(Media {
					merged_path: Some(summary.output_path.clone()),
					merge_backup_path: summary.backup_path.clone(),
					merge_state: MergeState::Merged,
					merge_detail: None,
					..media
				})
				.await?;
				(ACTIVITY_MERGED, summary.output_path.clone())
			}
			MergeOutcome::Skipped(reason) => {
				self.mark(media, MergeState::Skipped, Some(reason)).await?;
				(ACTIVITY_MERGE_SKIPPED, (*reason).to_string())
			}
		};
		self.activity
			.record(ActivityTarget::Media(&id), event, Some(&detail), None)
			.await?;
		Ok(outcome)
	}
}

fn revert_plan(media: &Media) -> Result<RevertPlan> {
	let (Some(merged_path), Some(backup_path)) =
		(media.merged_path.clone(), media.merge_backup_path.clone())
	else {
		return Err(Error::Validation(
			reasons::MERGE_NOTHING_TO_REVERT.to_string(),
		));
	};
	Ok(RevertPlan {
		source_dir: media.library_path.clone(),
		merged_path,
		backup_path,
	})
}
