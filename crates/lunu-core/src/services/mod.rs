mod activity;
mod api_key;
mod auth;
mod grab;
mod import;
mod invite;
mod issue;
mod job;
mod library;
mod media;
mod metadata;
mod monitor;
mod notification;
mod notification_inbox;
mod quality_profile;
mod release;
mod request;
mod settings;
mod user;

pub use activity::ActivityService;
pub use api_key::{ApiKeyService, IssuedApiKey};
pub use auth::{AuthService, Authenticated, Registration};
pub use grab::{GrabService, ReleaseSelection};
pub use import::ImportService;
pub use invite::{InviteService, IssuedInvite};
pub use issue::IssueService;
pub use job::JobService;
pub use library::{LibraryService, SyncSummary};
pub use media::MediaService;
pub use metadata::MetadataService;
pub use monitor::MonitorService;
pub use notification::{NotificationService, resolve_recipients};
pub use notification_inbox::NotificationInboxService;
pub use quality_profile::{QualityProfileInput, QualityProfileService};
pub use release::ReleaseService;
pub use request::{NewRequest, RequestService};
pub use settings::{SettingView, SettingsService};
pub use user::UserService;

use chrono::Utc;

use crate::consts::auth::PASSWORD_MIN_LEN;
use crate::consts::reasons;
use crate::crypto::hash_password;
use crate::models::{AuthSource, Role, User};
use crate::repo::UserRepo;
use crate::traits::ExternalIdentity;
use crate::{Error, Result};

pub(crate) fn validate_password(password: &str) -> Result<()> {
	if password.len() < PASSWORD_MIN_LEN {
		return Err(Error::Validation(reasons::PASSWORD_TOO_SHORT.to_string()));
	}
	Ok(())
}

pub(crate) fn normalize_email(email: Option<String>) -> Result<Option<String>> {
	let Some(value) = email else {
		return Ok(None);
	};
	let trimmed = value.trim();
	if trimmed.is_empty() {
		return Ok(None);
	}
	let valid = trimmed.split_once('@').is_some_and(|(local, domain)| {
		!local.is_empty()
			&& domain.contains('.')
			&& !domain.starts_with('.')
			&& !domain.ends_with('.')
	}) && !trimmed.contains(char::is_whitespace);
	if !valid {
		return Err(Error::Validation(reasons::EMAIL_INVALID.to_string()));
	}
	Ok(Some(trimmed.to_string()))
}

pub(crate) fn new_id() -> String {
	uuid::Uuid::new_v4().to_string()
}

pub(crate) fn nonempty(value: Option<String>) -> Option<String> {
	value
		.map(|value| value.trim().to_string())
		.filter(|value| !value.is_empty())
}

pub(crate) fn validate_locale(locale: Option<String>) -> Result<Option<String>> {
	let Some(value) = nonempty(locale) else {
		return Ok(None);
	};
	let resolved = lunu_i18n::resolve(&value)
		.ok_or_else(|| Error::Validation(reasons::UNKNOWN_LOCALE.to_string()))?;
	Ok(Some(resolved.to_string()))
}

pub(crate) async fn ensure_username_available(users: &dyn UserRepo, username: &str) -> Result<()> {
	if users.find_by_username(username).await?.is_some() {
		return Err(Error::Conflict(reasons::USERNAME_TAKEN.to_string()));
	}
	Ok(())
}

pub(crate) async fn require_user(users: &dyn UserRepo, id: &str) -> Result<User> {
	users
		.find_by_id(id)
		.await?
		.ok_or_else(|| Error::NotFound(format!("user {id}")))
}

pub(crate) fn build_external_user(identity: ExternalIdentity, role: Role) -> User {
	let now = Utc::now();
	User {
		id: new_id(),
		username: identity.username,
		email: identity.email,
		display_name: None,
		locale: None,
		password_hash: None,
		role,
		auth_source: AuthSource::Abs,
		enabled: true,
		email_verified: true,
		created_at: now,
		updated_at: now,
	}
}

pub(crate) fn build_local_user(
	username: &str,
	password: &str,
	email: Option<String>,
	role: Role,
) -> Result<User> {
	validate_password(password)?;
	let email = normalize_email(email)?;
	let now = Utc::now();
	Ok(User {
		id: new_id(),
		username: username.to_string(),
		email,
		display_name: None,
		locale: None,
		password_hash: Some(hash_password(password)?),
		role,
		auth_source: AuthSource::Local,
		enabled: true,
		email_verified: false,
		created_at: now,
		updated_at: now,
	})
}
