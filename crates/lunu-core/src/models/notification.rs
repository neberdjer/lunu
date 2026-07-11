use serde::{Deserialize, Serialize};

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
	pub fn summary(&self) -> &'static str {
		match self {
			NotificationKind::RequestPending => "New request pending approval",
			NotificationKind::RequestApproved => "Request approved",
			NotificationKind::RequestDeclined => "Request declined",
			NotificationKind::RequestAvailable => "Now available",
			NotificationKind::RequestFailed => "Request failed",
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
