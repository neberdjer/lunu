use chrono::{DateTime, Utc};
use lunu_core::models::{Issue, Job, UserNotification};
use serde::Serialize;

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct JobResponse {
	pub id: String,
	pub job_type: String,
	pub request_id: Option<String>,
	pub status: String,
	pub attempts: i64,
	pub max_attempts: i64,
	pub run_after: DateTime<Utc>,
	pub last_error: Option<String>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

impl From<&Job> for JobResponse {
	fn from(job: &Job) -> Self {
		Self {
			id: job.id.clone(),
			job_type: job.job_type.to_string(),
			request_id: job.request_id.clone(),
			status: job.status.to_string(),
			attempts: job.attempts,
			max_attempts: job.max_attempts,
			run_after: job.run_after,
			last_error: job.last_error.clone(),
			created_at: job.created_at,
			updated_at: job.updated_at,
		}
	}
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct IssueResponse {
	pub id: String,
	pub request_id: String,
	pub reporter_id: String,
	pub issue_type: String,
	pub detail: Option<String>,
	pub status: String,
	pub resolved_by: Option<String>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

impl From<&Issue> for IssueResponse {
	fn from(issue: &Issue) -> Self {
		Self {
			id: issue.id.clone(),
			request_id: issue.request_id.clone(),
			reporter_id: issue.reporter_id.clone(),
			issue_type: issue.issue_type.to_string(),
			detail: issue.detail.clone(),
			status: issue.status.to_string(),
			resolved_by: issue.resolved_by.clone(),
			created_at: issue.created_at,
			updated_at: issue.updated_at,
		}
	}
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct NotificationResponse {
	pub id: String,
	pub kind: String,
	pub summary: String,
	pub request_id: Option<String>,
	pub title: String,
	pub read: bool,
	pub created_at: DateTime<Utc>,
}

impl From<&UserNotification> for NotificationResponse {
	fn from(notification: &UserNotification) -> Self {
		Self {
			id: notification.id.clone(),
			kind: notification.kind.as_str().to_string(),
			summary: notification.kind.summary().to_string(),
			request_id: notification.request_id.clone(),
			title: notification.title.clone(),
			read: notification.read_at.is_some(),
			created_at: notification.created_at,
		}
	}
}
