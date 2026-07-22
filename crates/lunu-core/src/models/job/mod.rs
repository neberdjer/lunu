mod payloads;

pub use payloads::{GrabPayload, ImportPayload, MergePayload, MonitorPayload};

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Duration, Utc};

use crate::consts::jobs::{RETRY_BASE_SECS, RETRY_MAX_SECS};
use crate::consts::reasons;
use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
	Pending,
	Running,
	Completed,
	Failed,
}

impl JobStatus {
	pub fn as_str(&self) -> &'static str {
		match self {
			JobStatus::Pending => "pending",
			JobStatus::Running => "running",
			JobStatus::Completed => "completed",
			JobStatus::Failed => "failed",
		}
	}
}

impl fmt::Display for JobStatus {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

impl FromStr for JobStatus {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		match value {
			"pending" => Ok(JobStatus::Pending),
			"running" => Ok(JobStatus::Running),
			"completed" => Ok(JobStatus::Completed),
			"failed" => Ok(JobStatus::Failed),
			_ => Err(Error::Validation(reasons::JOB_STATUS_UNKNOWN.to_string())),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobType {
	Grab,
	MonitorDownload,
	Import,
	Merge,
	MergeRevert,
	Notify,
	LibrarySync,
	SessionCleanup,
	JobCleanup,
}

impl JobType {
	pub const ALL: &'static [JobType] = &[
		JobType::Grab,
		JobType::MonitorDownload,
		JobType::Import,
		JobType::Merge,
		JobType::MergeRevert,
		JobType::Notify,
		JobType::LibrarySync,
		JobType::SessionCleanup,
		JobType::JobCleanup,
	];

	pub fn media_lane() -> Vec<JobType> {
		Self::ALL
			.iter()
			.copied()
			.filter(JobType::media_subject)
			.collect()
	}

	pub fn general_lane() -> Vec<JobType> {
		Self::ALL
			.iter()
			.copied()
			.filter(|job_type| !job_type.media_subject())
			.collect()
	}

	pub fn as_str(&self) -> &'static str {
		match self {
			JobType::Grab => "grab",
			JobType::MonitorDownload => "monitor-download",
			JobType::Import => "import",
			JobType::Merge => "merge",
			JobType::MergeRevert => "merge-revert",
			JobType::Notify => "notify",
			JobType::LibrarySync => "library-sync",
			JobType::SessionCleanup => "session-cleanup",
			JobType::JobCleanup => "job-cleanup",
		}
	}

	pub fn is_recurring(&self) -> bool {
		matches!(
			self,
			JobType::LibrarySync | JobType::SessionCleanup | JobType::JobCleanup
		)
	}

	pub fn media_subject(&self) -> bool {
		match self {
			JobType::Merge | JobType::MergeRevert => true,
			JobType::Grab
			| JobType::MonitorDownload
			| JobType::Import
			| JobType::Notify
			| JobType::LibrarySync
			| JobType::SessionCleanup
			| JobType::JobCleanup => false,
		}
	}

	pub fn propagates_failure_to_request(&self) -> bool {
		match self {
			JobType::Grab | JobType::MonitorDownload | JobType::Import => true,
			JobType::Merge
			| JobType::MergeRevert
			| JobType::Notify
			| JobType::LibrarySync
			| JobType::SessionCleanup
			| JobType::JobCleanup => false,
		}
	}
}

impl fmt::Display for JobType {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

impl FromStr for JobType {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		match value {
			"grab" => Ok(JobType::Grab),
			"monitor-download" => Ok(JobType::MonitorDownload),
			"import" => Ok(JobType::Import),
			"merge" => Ok(JobType::Merge),
			"merge-revert" => Ok(JobType::MergeRevert),
			"notify" => Ok(JobType::Notify),
			"library-sync" => Ok(JobType::LibrarySync),
			"session-cleanup" => Ok(JobType::SessionCleanup),
			"job-cleanup" => Ok(JobType::JobCleanup),
			_ => Err(Error::Validation(reasons::JOB_TYPE_UNKNOWN.to_string())),
		}
	}
}

#[derive(Debug, Clone)]
pub struct Job {
	pub id: String,
	pub job_type: JobType,
	pub request_id: Option<String>,
	pub dedupe_key: Option<String>,
	pub payload: String,
	pub status: JobStatus,
	pub attempts: i64,
	pub max_attempts: i64,
	pub run_after: DateTime<Utc>,
	pub locked_by: Option<String>,
	pub locked_at: Option<DateTime<Utc>>,
	pub last_error: Option<String>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

impl Job {
	pub fn should_retry(&self) -> bool {
		self.attempts < self.max_attempts
	}

	pub fn retry_backoff(&self) -> Duration {
		let shift = (self.attempts - 1).clamp(0, 16) as u32;
		let seconds = RETRY_BASE_SECS
			.saturating_mul(1_i64 << shift)
			.min(RETRY_MAX_SECS);
		Duration::seconds(seconds)
	}
}

#[cfg(test)]
mod tests;
