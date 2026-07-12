use chrono::{DateTime, Utc};
use lunu_core::models::{
	Activity, ApiKey, BlocklistEntry, Download, Invite, Issue, Job, QualityProfile, Request,
	Session, User, UserNotification, UserSettings,
};
use serde::Serialize;

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct BlocklistResponse {
	pub id: String,
	pub request_id: String,
	pub download_url: String,
	pub created_at: DateTime<Utc>,
}

impl From<&BlocklistEntry> for BlocklistResponse {
	fn from(entry: &BlocklistEntry) -> Self {
		Self {
			id: entry.id.clone(),
			request_id: entry.request_id.clone(),
			download_url: entry.download_url.clone(),
			created_at: entry.created_at,
		}
	}
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct ActivityResponse {
	pub id: String,
	pub request_id: String,
	pub event: String,
	pub detail: Option<String>,
	pub actor: Option<String>,
	pub at: DateTime<Utc>,
}

impl From<&Activity> for ActivityResponse {
	fn from(activity: &Activity) -> Self {
		Self {
			id: activity.id.clone(),
			request_id: activity.request_id.clone(),
			event: activity.event.clone(),
			detail: activity.detail.clone(),
			actor: activity.actor.clone(),
			at: activity.at,
		}
	}
}

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
pub(crate) struct UserResponse {
	pub id: String,
	pub username: String,
	pub email: Option<String>,
	pub display_name: Option<String>,
	pub role: String,
	pub auth_source: String,
	pub enabled: bool,
	pub created_at: DateTime<Utc>,
}

impl From<&User> for UserResponse {
	fn from(user: &User) -> Self {
		Self {
			id: user.id.clone(),
			username: user.username.clone(),
			email: user.email.clone(),
			display_name: user.display_name.clone(),
			role: user.role.to_string(),
			auth_source: user.auth_source.to_string(),
			enabled: user.enabled,
			created_at: user.created_at,
		}
	}
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct ApiKeyResponse {
	pub id: String,
	pub name: String,
	pub prefix: String,
	pub scopes: Vec<String>,
	pub created_at: DateTime<Utc>,
	pub last_used_at: Option<DateTime<Utc>>,
	pub expires_at: Option<DateTime<Utc>>,
	pub revoked: bool,
}

impl From<&ApiKey> for ApiKeyResponse {
	fn from(key: &ApiKey) -> Self {
		Self {
			id: key.id.clone(),
			name: key.name.clone(),
			prefix: key.prefix.clone(),
			scopes: key.scopes.clone(),
			created_at: key.created_at,
			last_used_at: key.last_used_at,
			expires_at: key.expires_at,
			revoked: key.revoked,
		}
	}
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct RequestResponse {
	pub id: String,
	pub user_id: String,
	pub asin: String,
	pub title: String,
	pub author: Option<String>,
	pub cover_url: Option<String>,
	pub status: String,
	pub approved_by: Option<String>,
	pub notes: Option<String>,
	pub quality_profile_id: Option<String>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

impl From<&Request> for RequestResponse {
	fn from(request: &Request) -> Self {
		Self {
			id: request.id.clone(),
			user_id: request.user_id.clone(),
			asin: request.asin.clone(),
			title: request.title.clone(),
			author: request.author.clone(),
			cover_url: request.cover_url.clone(),
			status: request.status.to_string(),
			approved_by: request.approved_by.clone(),
			notes: request.notes.clone(),
			quality_profile_id: request.quality_profile_id.clone(),
			created_at: request.created_at,
			updated_at: request.updated_at,
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
pub(crate) struct SessionResponse {
	pub id: String,
	pub current: bool,
	pub user_agent: Option<String>,
	pub created_at: DateTime<Utc>,
	pub expires_at: DateTime<Utc>,
	pub last_seen_at: Option<DateTime<Utc>>,
}

impl SessionResponse {
	pub fn new(session: &Session, current: bool) -> Self {
		Self {
			id: session.id.clone(),
			current,
			user_agent: session.user_agent.clone(),
			created_at: session.created_at,
			expires_at: session.expires_at,
			last_seen_at: session.last_seen_at,
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

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct UserSettingsResponse {
	pub user_id: String,
	pub auto_approve: bool,
	pub request_quota: Option<i64>,
	pub quota_days: Option<i64>,
}

impl From<&UserSettings> for UserSettingsResponse {
	fn from(settings: &UserSettings) -> Self {
		Self {
			user_id: settings.user_id.clone(),
			auto_approve: settings.auto_approve,
			request_quota: settings.request_quota,
			quota_days: settings.quota_days,
		}
	}
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct DownloadResponse {
	pub id: String,
	pub request_id: String,
	pub client: String,
	pub category: String,
	pub release_title: String,
	pub indexer: String,
	pub download_url: String,
	pub info_hash: Option<String>,
	pub state: String,
	pub progress: i64,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

impl From<&Download> for DownloadResponse {
	fn from(download: &Download) -> Self {
		Self {
			id: download.id.clone(),
			request_id: download.request_id.clone(),
			client: download.client.clone(),
			category: download.category.clone(),
			release_title: download.release_title.clone(),
			indexer: download.indexer.clone(),
			download_url: download.download_url.clone(),
			info_hash: download.info_hash.clone(),
			state: download.state.to_string(),
			progress: download.progress,
			created_at: download.created_at,
			updated_at: download.updated_at,
		}
	}
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct QualityProfileResponse {
	pub id: String,
	pub name: String,
	pub allowed_formats: Vec<String>,
	pub preferred_formats: Vec<String>,
	pub min_seeders: i64,
	pub min_size_mb: Option<i64>,
	pub max_size_mb: Option<i64>,
	pub seeder_weight: i64,
	pub format_weight: i64,
	pub is_default: bool,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

impl From<&QualityProfile> for QualityProfileResponse {
	fn from(profile: &QualityProfile) -> Self {
		Self {
			id: profile.id.clone(),
			name: profile.name.clone(),
			allowed_formats: profile.allowed_formats.clone(),
			preferred_formats: profile.preferred_formats.clone(),
			min_seeders: profile.min_seeders,
			min_size_mb: profile.min_size_mb,
			max_size_mb: profile.max_size_mb,
			seeder_weight: profile.seeder_weight,
			format_weight: profile.format_weight,
			is_default: profile.is_default,
			created_at: profile.created_at,
			updated_at: profile.updated_at,
		}
	}
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct InviteResponse {
	pub id: String,
	pub role: String,
	pub email: Option<String>,
	pub created_by: String,
	pub max_uses: i64,
	pub used_count: i64,
	pub created_at: DateTime<Utc>,
	pub expires_at: Option<DateTime<Utc>>,
}

impl From<&Invite> for InviteResponse {
	fn from(invite: &Invite) -> Self {
		Self {
			id: invite.id.clone(),
			role: invite.role.to_string(),
			email: invite.email.clone(),
			created_by: invite.created_by.clone(),
			max_uses: invite.max_uses,
			used_count: invite.used_count,
			created_at: invite.created_at,
			expires_at: invite.expires_at,
		}
	}
}
