use async_trait::async_trait;

use crate::Result;
use crate::models::SourceDisposition;

pub struct MergePlan {
	pub source_dir: String,
	pub output_path: String,
	pub title: String,
	pub author: Option<String>,
	pub series: Option<String>,
	pub sequence: Option<String>,
	pub bitrate: String,
	pub sources: SourceDisposition,
	pub previous_output: Option<String>,
}

pub struct RevertPlan {
	pub source_dir: String,
	pub merged_path: String,
	pub backup_path: String,
}

pub struct MergeSummary {
	pub output_path: String,
	pub backup_path: Option<String>,
	pub chapters: usize,
	pub handled_sources: usize,
}

pub enum MergeOutcome {
	Merged(MergeSummary),
	Skipped(&'static str),
}

pub struct PreviewChapter {
	pub title: String,
	pub seconds: f64,
}

pub struct MergePreview {
	pub output_path: String,
	pub skip: Option<&'static str>,
	pub chapters: Vec<PreviewChapter>,
	pub total_seconds: f64,
	pub copyable: bool,
	pub sources: SourceDisposition,
	pub bitrate: String,
}

impl Default for MergePreview {
	fn default() -> Self {
		Self {
			output_path: String::new(),
			skip: None,
			chapters: Vec::new(),
			total_seconds: 0.0,
			copyable: false,
			sources: SourceDisposition::Keep,
			bitrate: String::new(),
		}
	}
}

impl MergePreview {
	pub fn skipped(reason: &'static str) -> Self {
		Self {
			skip: Some(reason),
			..Self::default()
		}
	}

	pub fn of(plan: &MergePlan) -> Self {
		Self {
			output_path: plan.output_path.clone(),
			sources: plan.sources.clone(),
			bitrate: plan.bitrate.clone(),
			..Self::default()
		}
	}
}

#[async_trait]
pub trait Merger: Send + Sync {
	async fn test_connection(&self) -> Result<()>;
	async fn merge(&self, plan: &MergePlan) -> Result<MergeOutcome>;
	async fn preview(&self, plan: &MergePlan) -> Result<MergePreview>;
	async fn revert(&self, plan: &RevertPlan) -> Result<usize>;
}
