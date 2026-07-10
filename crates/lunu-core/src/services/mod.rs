mod api_key;
mod auth;
mod invite;
mod metadata;
mod settings;
mod user;

pub use api_key::{ApiKeyService, IssuedApiKey};
pub use auth::{AuthService, Authenticated};
pub use invite::{InviteService, IssuedInvite};
pub use metadata::MetadataService;
pub use settings::SettingsService;
pub use user::UserService;

use chrono::Utc;

use crate::crypto::hash_password;
use crate::models::{AuthSource, Role, User};
use crate::repo::UserRepo;
use crate::{Error, Result};

pub(crate) fn new_id() -> String {
	uuid::Uuid::new_v4().to_string()
}

pub(crate) async fn ensure_username_available(users: &dyn UserRepo, username: &str) -> Result<()> {
	if users.find_by_username(username).await?.is_some() {
		return Err(Error::Conflict("username-taken".to_string()));
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
