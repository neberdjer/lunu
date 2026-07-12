use std::sync::Arc;

use chrono::Utc;
use tokio::sync::{Mutex, MutexGuard};

use crate::consts::reasons;
use crate::crypto::hash_password;
use crate::models::{AuthSource, Role, User, UserSettings};
use crate::repo::{SessionRepo, UserRepo, UserSettingsRepo};
use crate::services::{
	build_local_user, ensure_username_available, nonempty, normalize_email, require_user,
	validate_locale, validate_password,
};
use crate::{Error, Result};

pub struct UserService {
	users: Arc<dyn UserRepo>,
	sessions: Arc<dyn SessionRepo>,
	settings: Arc<dyn UserSettingsRepo>,
	admin_lock: Mutex<()>,
}

impl UserService {
	pub fn new(
		users: Arc<dyn UserRepo>,
		sessions: Arc<dyn SessionRepo>,
		settings: Arc<dyn UserSettingsRepo>,
	) -> Self {
		Self {
			users,
			sessions,
			settings,
			admin_lock: Mutex::new(()),
		}
	}

	pub async fn get_settings(&self, user_id: &str) -> Result<Option<UserSettings>> {
		self.settings.get(user_id).await
	}

	pub async fn set_settings(
		&self,
		user_id: &str,
		auto_approve: bool,
		request_quota: Option<i64>,
		quota_days: Option<i64>,
	) -> Result<UserSettings> {
		require_user(self.users.as_ref(), user_id).await?;

		let settings = UserSettings {
			user_id: user_id.to_string(),
			auto_approve,
			request_quota,
			quota_days,
			updated_at: Utc::now(),
		};
		self.settings.upsert(&settings).await?;
		Ok(settings)
	}

	pub async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<User>> {
		self.users.list_page(limit, offset).await
	}

	pub async fn count(&self) -> Result<i64> {
		self.users.count().await
	}

	pub async fn get(&self, id: &str) -> Result<Option<User>> {
		self.users.find_by_id(id).await
	}

	pub async fn create(
		&self,
		username: &str,
		password: &str,
		email: Option<String>,
		role: Role,
	) -> Result<User> {
		ensure_username_available(self.users.as_ref(), username).await?;

		let user = build_local_user(username, password, email, role)?;
		self.users.create(&user).await?;
		Ok(user)
	}

	pub async fn update_profile(
		&self,
		id: &str,
		email: Option<String>,
		display_name: Option<String>,
		locale: Option<String>,
	) -> Result<User> {
		let mut user = require_user(self.users.as_ref(), id).await?;

		user.email = normalize_email(email)?;
		user.display_name = nonempty(display_name);
		user.locale = validate_locale(locale)?;
		user.updated_at = Utc::now();
		self.users.update(&user).await?;
		Ok(user)
	}

	pub async fn set_enabled(&self, id: &str, enabled: bool) -> Result<User> {
		self.admin_update(id, Some(enabled), None, None, None).await
	}

	pub async fn admin_update(
		&self,
		id: &str,
		enabled: Option<bool>,
		role: Option<Role>,
		display_name: Option<Option<String>>,
		locale: Option<Option<String>>,
	) -> Result<User> {
		let mut user = require_user(self.users.as_ref(), id).await?;

		let new_enabled = enabled.unwrap_or(user.enabled);
		let new_role = role.unwrap_or(user.role);
		let losing_admin =
			user.role.is_admin() && user.enabled && !(new_role.is_admin() && new_enabled);
		let _guard = self.guard_admin_removal(&user, losing_admin).await?;

		user.enabled = new_enabled;
		user.role = new_role;
		if let Some(display_name) = display_name {
			user.display_name = nonempty(display_name);
		}
		if let Some(locale) = locale {
			user.locale = validate_locale(locale)?;
		}
		user.updated_at = Utc::now();
		self.users.update(&user).await?;

		if !user.enabled {
			self.sessions.delete_for_user(id).await?;
		}

		Ok(user)
	}

	pub async fn set_password(&self, id: &str, password: &str) -> Result<User> {
		let mut user = require_user(self.users.as_ref(), id).await?;
		if user.auth_source != AuthSource::Local {
			return Err(Error::Validation(reasons::PASSWORD_NOT_LOCAL.to_string()));
		}
		validate_password(password)?;

		user.password_hash = Some(hash_password(password)?);
		user.updated_at = Utc::now();
		self.users.update(&user).await?;
		self.sessions.delete_for_user(id).await?;
		Ok(user)
	}

	pub async fn delete(&self, id: &str) -> Result<()> {
		let user = require_user(self.users.as_ref(), id).await?;
		let _guard = self.guard_admin_removal(&user, true).await?;

		self.sessions.delete_for_user(id).await?;
		self.settings.delete(id).await?;
		self.users.delete(id).await
	}

	async fn guard_admin_removal(
		&self,
		user: &User,
		removing: bool,
	) -> Result<Option<MutexGuard<'_, ()>>> {
		if !(removing && user.role.is_admin() && user.enabled) {
			return Ok(None);
		}

		let guard = self.admin_lock.lock().await;
		if self.users.count_enabled_admins_excluding(&user.id).await? == 0 {
			return Err(Error::Conflict(reasons::LAST_ADMIN.to_string()));
		}
		Ok(Some(guard))
	}
}
