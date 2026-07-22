use chrono::Utc;

use super::AuthService;
use crate::consts::auth::SESSION_TOUCH_INTERVAL_SECS;
use crate::crypto::hash_token;
use crate::models::{Session, User};
use crate::services::stale;
use crate::{Error, Result};

impl AuthService {
	pub async fn validate_session(&self, token: &str) -> Result<Option<User>> {
		let Some(session) = self.sessions.find_by_token_hash(&hash_token(token)).await? else {
			return Ok(None);
		};

		let now = Utc::now();
		if session.is_expired(now) {
			self.sessions.delete(&session.id).await?;
			return Ok(None);
		}

		let Some(user) = self.users.find_by_id(&session.user_id).await? else {
			self.sessions.delete(&session.id).await?;
			return Ok(None);
		};

		if !user.enabled {
			return Ok(None);
		}

		if stale(session.last_seen_at, now, SESSION_TOUCH_INTERVAL_SECS) {
			self.sessions.touch(&session.id, now).await?;
		}
		Ok(Some(user))
	}

	pub async fn cleanup_expired_sessions(&self) -> Result<()> {
		self.sessions.delete_expired(chrono::Utc::now()).await
	}

	pub async fn list_sessions_page(
		&self,
		user_id: &str,
		limit: i64,
		offset: i64,
	) -> Result<Vec<Session>> {
		self.sessions
			.list_for_user_page(user_id, limit, offset)
			.await
	}

	pub async fn count_sessions(&self, user_id: &str) -> Result<i64> {
		self.sessions.count_for_user(user_id).await
	}

	pub async fn current_session_id(&self, token: &str) -> Result<Option<String>> {
		Ok(self
			.sessions
			.find_by_token_hash(&hash_token(token))
			.await?
			.map(|session| session.id))
	}

	pub async fn revoke_session(&self, user_id: &str, id: &str) -> Result<()> {
		if !self.sessions.delete_scoped(user_id, id).await? {
			return Err(Error::NotFound(format!("session {id}")));
		}
		Ok(())
	}
}
