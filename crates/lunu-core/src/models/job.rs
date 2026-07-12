use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::consts::jobs::{RETRY_BASE_SECS, RETRY_MAX_SECS};
use crate::consts::reasons;
use crate::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrabPayload {
	pub request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorPayload {
	pub download_id: String,
	pub misses: i64,
	#[serde(default)]
	pub stalls: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPayload {
	pub download_id: String,
	pub content_path: String,
}

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
	Search,
	Grab,
	MonitorDownload,
	Import,
	Finalize,
	Notify,
}

impl JobType {
	pub fn as_str(&self) -> &'static str {
		match self {
			JobType::Search => "search",
			JobType::Grab => "grab",
			JobType::MonitorDownload => "monitor-download",
			JobType::Import => "import",
			JobType::Finalize => "finalize",
			JobType::Notify => "notify",
		}
	}

	pub fn propagates_failure_to_request(&self) -> bool {
		match self {
			JobType::Search
			| JobType::Grab
			| JobType::MonitorDownload
			| JobType::Import
			| JobType::Finalize => true,
			JobType::Notify => false,
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
			"search" => Ok(JobType::Search),
			"grab" => Ok(JobType::Grab),
			"monitor-download" => Ok(JobType::MonitorDownload),
			"import" => Ok(JobType::Import),
			"finalize" => Ok(JobType::Finalize),
			"notify" => Ok(JobType::Notify),
			_ => Err(Error::Validation(reasons::JOB_TYPE_UNKNOWN.to_string())),
		}
	}
}

#[derive(Debug, Clone)]
pub struct Job {
	pub id: String,
	pub job_type: JobType,
	pub request_id: Option<String>,
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
mod tests {
	use super::*;

	fn job(attempts: i64, max_attempts: i64) -> Job {
		let now = Utc::now();
		Job {
			id: "j1".to_string(),
			job_type: JobType::Search,
			request_id: None,
			payload: "{}".to_string(),
			status: JobStatus::Running,
			attempts,
			max_attempts,
			run_after: now,
			locked_by: None,
			locked_at: None,
			last_error: None,
			created_at: now,
			updated_at: now,
		}
	}

	#[test]
	fn should_retry_until_max_attempts() {
		assert!(job(1, 3).should_retry());
		assert!(job(2, 3).should_retry());
		assert!(!job(3, 3).should_retry());
		assert!(!job(4, 3).should_retry());
	}

	#[test]
	fn retry_backoff_grows_then_caps() {
		assert_eq!(
			job(1, 5).retry_backoff(),
			Duration::seconds(RETRY_BASE_SECS)
		);
		assert_eq!(
			job(2, 5).retry_backoff(),
			Duration::seconds(RETRY_BASE_SECS * 2)
		);
		assert_eq!(
			job(3, 5).retry_backoff(),
			Duration::seconds(RETRY_BASE_SECS * 4)
		);
		assert!(job(40, 50).retry_backoff() <= Duration::seconds(RETRY_MAX_SECS));
	}
}
