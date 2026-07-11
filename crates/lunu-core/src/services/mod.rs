mod activity;
mod api_key;
mod auth;
mod grab;
mod import;
mod invite;
mod job;
mod metadata;
mod monitor;
mod quality_profile;
mod release;
mod request;
mod settings;
mod user;

pub use activity::ActivityService;
pub use api_key::{ApiKeyService, IssuedApiKey};
pub use auth::{AuthService, Authenticated};
pub use grab::{GrabService, ReleaseSelection};
pub use import::ImportService;
pub use invite::{InviteService, IssuedInvite};
pub use job::JobService;
pub use metadata::MetadataService;
pub use monitor::MonitorService;
pub use quality_profile::{QualityProfileInput, QualityProfileService};
pub use release::ReleaseService;
pub use request::RequestService;
pub use settings::{SettingView, SettingsService};
pub use user::UserService;

use chrono::Utc;

use crate::consts::reasons;
use crate::crypto::hash_password;
use crate::models::{AuthSource, Role, User};
use crate::repo::UserRepo;
use crate::{Error, Result};

pub(crate) fn new_id() -> String {
	uuid::Uuid::new_v4().to_string()
}

pub(crate) async fn ensure_username_available(users: &dyn UserRepo, username: &str) -> Result<()> {
	if users.find_by_username(username).await?.is_some() {
		return Err(Error::Conflict(reasons::USERNAME_TAKEN.to_string()));
	}
	Ok(())
}

pub(crate) fn build_local_user(
	username: &str,
	password: &str,
	email: Option<String>,
	role: Role,
) -> Result<User> {
	let now = Utc::now();
	Ok(User {
		id: new_id(),
		username: username.to_string(),
		email,
		password_hash: Some(hash_password(password)?),
		role,
		auth_source: AuthSource::Local,
		enabled: true,
		created_at: now,
		updated_at: now,
	})
}
