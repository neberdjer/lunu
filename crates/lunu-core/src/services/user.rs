use std::sync::Arc;

use chrono::Utc;

use crate::models::{Role, User, UserSettings};
use crate::repo::{SessionRepo, UserRepo, UserSettingsRepo};
use crate::services::{build_local_user, ensure_username_available};
use crate::{Error, Result};

pub struct UserService {
	users: Arc<dyn UserRepo>,
	sessions: Arc<dyn SessionRepo>,
	settings: Arc<dyn UserSettingsRepo>,
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
		if self.users.find_by_id(user_id).await?.is_none() {
			return Err(Error::NotFound(format!("user {user_id}")));
		}

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

	pub async fn set_enabled(&self, id: &str, enabled: bool) -> Result<User> {
		let mut user = self
			.users
			.find_by_id(id)
			.await?
			.ok_or_else(|| Error::NotFound(format!("user {id}")))?;

		user.enabled = enabled;
		user.updated_at = Utc::now();
		self.users.update(&user).await?;

		if !enabled {
			self.sessions.delete_for_user(id).await?;
		}

		Ok(user)
	}

	pub async fn delete(&self, id: &str) -> Result<()> {
		self.sessions.delete_for_user(id).await?;
		self.settings.delete(id).await?;
		self.users.delete(id).await
	}
}
