use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::consts::reasons;
use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NotificationKind {
	RequestPending,
	RequestApproved,
	RequestDeclined,
	RequestAvailable,
	RequestFailed,
}

impl NotificationKind {
	pub fn summary(&self) -> String {
		lunu_i18n::t(
			&lunu_i18n::default_locale(),
			&format!("notification-{}", self.as_str()),
		)
	}

	pub fn as_str(&self) -> &'static str {
		match self {
			NotificationKind::RequestPending => "request-pending",
			NotificationKind::RequestApproved => "request-approved",
			NotificationKind::RequestDeclined => "request-declined",
			NotificationKind::RequestAvailable => "request-available",
			NotificationKind::RequestFailed => "request-failed",
		}
	}
}

impl fmt::Display for NotificationKind {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

impl FromStr for NotificationKind {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		match value {
			"request-pending" => Ok(NotificationKind::RequestPending),
			"request-approved" => Ok(NotificationKind::RequestApproved),
			"request-declined" => Ok(NotificationKind::RequestDeclined),
			"request-available" => Ok(NotificationKind::RequestAvailable),
			"request-failed" => Ok(NotificationKind::RequestFailed),
			_ => Err(Error::Validation(
				reasons::NOTIFICATION_KIND_UNKNOWN.to_string(),
			)),
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationEvent {
	pub kind: NotificationKind,
	pub request_id: String,
	pub title: String,
	pub user_id: String,
}

impl NotificationEvent {
	pub fn message(&self) -> String {
		format!("{}: {}", self.kind.summary(), self.title)
	}
}
