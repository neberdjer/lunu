use std::sync::Arc;

use chrono::Utc;

use crate::models::{Role, User};
use crate::repo::{SessionRepo, UserRepo};
use crate::services::{build_local_user, ensure_username_available};
use crate::{Error, Result};

pub struct UserService {
	users: Arc<dyn UserRepo>,
	sessions: Arc<dyn SessionRepo>,
}

impl UserService {
	pub fn new(users: Arc<dyn UserRepo>, sessions: Arc<dyn SessionRepo>) -> Self {
		Self { users, sessions }
	}

	pub async fn list(&self) -> Result<Vec<User>> {
		self.users.list().await
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
		self.users.delete(id).await
	}
}
