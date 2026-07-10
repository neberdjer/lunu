use chrono::{DateTime, Utc};
use lunu_core::models::{ApiKey, Invite, User};
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct UserResponse {
	pub id: String,
	pub username: String,
	pub email: Option<String>,
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
			role: user.role.to_string(),
			auth_source: user.auth_source.to_string(),
			enabled: user.enabled,
			created_at: user.created_at,
		}
	}
}

#[derive(Serialize)]
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

#[derive(Serialize)]
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
