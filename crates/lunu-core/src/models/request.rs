use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};

use crate::consts::reasons;
use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestStatus {
	Pending,
	Approved,
	Declined,
	Downloading,
	Importing,
	Available,
	Failed,
}

impl RequestStatus {
	pub fn as_str(&self) -> &'static str {
		match self {
			RequestStatus::Pending => "pending",
			RequestStatus::Approved => "approved",
			RequestStatus::Declined => "declined",
			RequestStatus::Downloading => "downloading",
			RequestStatus::Importing => "importing",
			RequestStatus::Available => "available",
			RequestStatus::Failed => "failed",
		}
	}

	pub fn is_pending(&self) -> bool {
		matches!(self, RequestStatus::Pending)
	}

	pub fn is_reopenable(&self) -> bool {
		matches!(self, RequestStatus::Declined | RequestStatus::Failed)
	}

	pub fn allows_issue(&self) -> bool {
		matches!(self, RequestStatus::Available)
	}
}

impl fmt::Display for RequestStatus {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

impl FromStr for RequestStatus {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		match value {
			"pending" => Ok(RequestStatus::Pending),
			"approved" => Ok(RequestStatus::Approved),
			"declined" => Ok(RequestStatus::Declined),
			"downloading" => Ok(RequestStatus::Downloading),
			"importing" => Ok(RequestStatus::Importing),
			"available" => Ok(RequestStatus::Available),
			"failed" => Ok(RequestStatus::Failed),
			_ => Err(Error::Validation(
				reasons::REQUEST_STATUS_UNKNOWN.to_string(),
			)),
		}
	}
}

#[derive(Debug, Clone)]
pub struct Request {
	pub id: String,
	pub user_id: String,
	pub asin: Option<String>,
	pub title: String,
	pub author: Option<String>,
	pub cover_url: Option<String>,
	pub status: RequestStatus,
	pub approved_by: Option<String>,
	pub notes: Option<String>,
	pub quality_profile_id: Option<String>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}
