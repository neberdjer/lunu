use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};

use crate::consts::reasons;
use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadState {
	Queued,
	Downloading,
	Completed,
	Failed,
}

impl DownloadState {
	pub fn as_str(&self) -> &'static str {
		match self {
			DownloadState::Queued => "queued",
			DownloadState::Downloading => "downloading",
			DownloadState::Completed => "completed",
			DownloadState::Failed => "failed",
		}
	}
}

impl fmt::Display for DownloadState {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

impl FromStr for DownloadState {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		match value {
			"queued" => Ok(DownloadState::Queued),
			"downloading" => Ok(DownloadState::Downloading),
			"completed" => Ok(DownloadState::Completed),
			"failed" => Ok(DownloadState::Failed),
			_ => Err(Error::Validation(
				reasons::DOWNLOAD_STATE_UNKNOWN.to_string(),
			)),
		}
	}
}

#[derive(Debug, Clone)]
pub struct Download {
	pub id: String,
	pub request_id: String,
	pub client: String,
	pub category: String,
	pub release_title: String,
	pub indexer: String,
	pub download_url: String,
	pub client_ref: Option<String>,
	pub state: DownloadState,
	pub progress: i64,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DownloadStatus {
	pub state: DownloadState,
	pub progress: f64,
	pub content_path: Option<String>,
}
