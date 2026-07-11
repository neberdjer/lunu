use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::consts::auth::SESSION_TTL_DAYS;
use crate::consts::reasons;
use crate::crypto::{generate_token, hash_token, verify_password};
use crate::models::{Role, Session, User};
use crate::repo::{InviteRepo, SessionRepo, UserRepo};
use crate::services::{build_local_user, ensure_username_available, new_id};
use crate::{Error, Result};

pub struct Authenticated {
	pub user: User,
	pub session_token: String,
}

pub struct AuthService {
	users: Arc<dyn UserRepo>,
	sessions: Arc<dyn SessionRepo>,
	invites: Arc<dyn InviteRepo>,
}

impl AuthService {
	pub fn new(
		users: Arc<dyn UserRepo>,
		sessions: Arc<dyn SessionRepo>,
		invites: Arc<dyn InviteRepo>,
	) -> Self {
		Self {
			users,
			sessions,
			invites,
		}
	}

	pub async fn needs_setup(&self) -> Result<bool> {
		Ok(self.users.count().await? == 0)
	}

	pub async fn setup_first_admin(
		&self,
		username: &str,
		password: &str,
		email: Option<String>,
	) -> Result<Authenticated> {
		if self.users.count().await? > 0 {
			return Err(Error::Conflict(reasons::SETUP_COMPLETED.to_string()));
		}

		let user = build_local_user(username, password, email, Role::Admin)?;
		self.users.create(&user).await?;

		let session_token = self.create_session(&user.id).await?;
		Ok(Authenticated {
			user,
			session_token,
		})
	}

	pub async fn login(&self, username: &str, password: &str) -> Result<Authenticated> {
		let user = self
			.users
			.find_by_username(username)
			.await?
			.ok_or(Error::Unauthorized)?;

		if !user.enabled {
			return Err(Error::Forbidden);
		}

		let hash = user.password_hash.as_deref().ok_or(Error::Unauthorized)?;
		if !verify_password(password, hash)? {
			return Err(Error::Unauthorized);
		}

		let session_token = self.create_session(&user.id).await?;
		Ok(Authenticated {
			user,
			session_token,
		})
	}

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

		self.sessions.touch(&session.id, now).await?;
		Ok(Some(user))
	}

	pub async fn logout(&self, token: &str) -> Result<()> {
		if let Some(session) = self.sessions.find_by_token_hash(&hash_token(token)).await? {
			self.sessions.delete(&session.id).await?;
		}
		Ok(())
	}

	pub async fn register_with_invite(
		&self,
		code: &str,
		username: &str,
		password: &str,
	) -> Result<Authenticated> {
		let invite = self
			.invites
			.find_by_code_hash(&hash_token(code))
			.await?
			.ok_or_else(|| Error::Validation(reasons::INVITE_INVALID.to_string()))?;

		if !invite.is_redeemable(Utc::now()) {
			return Err(Error::Validation(reasons::INVITE_UNUSABLE.to_string()));
		}

		ensure_username_available(self.users.as_ref(), username).await?;

		let user = build_local_user(username, password, invite.email.clone(), invite.role)?;
		self.users.create(&user).await?;
		self.invites.increment_used(&invite.id).await?;

		let session_token = self.create_session(&user.id).await?;
		Ok(Authenticated {
			user,
			session_token,
		})
	}

	async fn create_session(&self, user_id: &str) -> Result<String> {
		let token = generate_token();
		let now = Utc::now();
		let session = Session {
			id: new_id(),
			user_id: user_id.to_string(),
			token_hash: hash_token(&token),
			created_at: now,
			expires_at: now + Duration::days(SESSION_TTL_DAYS),
			last_seen_at: None,
			user_agent: None,
		};

		self.sessions.create(&session).await?;
		Ok(token)
	}
}
