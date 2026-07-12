use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};

use crate::consts::reasons;
use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueType {
	WrongBook,
	BadQuality,
	Incomplete,
	Corrupt,
	Other,
}

impl IssueType {
	pub fn as_str(&self) -> &'static str {
		match self {
			IssueType::WrongBook => "wrong-book",
			IssueType::BadQuality => "bad-quality",
			IssueType::Incomplete => "incomplete",
			IssueType::Corrupt => "corrupt",
			IssueType::Other => "other",
		}
	}
}

impl fmt::Display for IssueType {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

impl FromStr for IssueType {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		match value {
			"wrong-book" => Ok(IssueType::WrongBook),
			"bad-quality" => Ok(IssueType::BadQuality),
			"incomplete" => Ok(IssueType::Incomplete),
			"corrupt" => Ok(IssueType::Corrupt),
			"other" => Ok(IssueType::Other),
			_ => Err(Error::Validation(reasons::ISSUE_TYPE_UNKNOWN.to_string())),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueStatus {
	Open,
	Resolved,
}

impl IssueStatus {
	pub fn as_str(&self) -> &'static str {
		match self {
			IssueStatus::Open => "open",
			IssueStatus::Resolved => "resolved",
		}
	}

	pub fn is_open(&self) -> bool {
		matches!(self, IssueStatus::Open)
	}
}

impl fmt::Display for IssueStatus {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

impl FromStr for IssueStatus {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		match value {
			"open" => Ok(IssueStatus::Open),
			"resolved" => Ok(IssueStatus::Resolved),
			_ => Err(Error::Validation(reasons::ISSUE_STATUS_UNKNOWN.to_string())),
		}
	}
}

#[derive(Debug, Clone)]
pub struct Issue {
	pub id: String,
	pub request_id: String,
	pub reporter_id: String,
	pub issue_type: IssueType,
	pub detail: Option<String>,
	pub status: IssueStatus,
	pub resolved_by: Option<String>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}
