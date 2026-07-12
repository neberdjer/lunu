use chrono::{DateTime, Utc};
use lunu_core::models::{ApiKey, Invite, Session, User, UserSettings};
use serde::Serialize;

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct UserResponse {
	pub id: String,
	pub username: String,
	pub email: Option<String>,
	pub display_name: Option<String>,
	pub locale: Option<String>,
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
			locale: user.locale.clone(),
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
