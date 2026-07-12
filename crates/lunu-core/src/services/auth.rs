use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::consts::auth::{
	PASSWORD_RESET_CODE_DIGITS, PASSWORD_RESET_COOLDOWN_SECONDS, PASSWORD_RESET_MAX_ATTEMPTS,
	PASSWORD_RESET_TTL_MINUTES, SESSION_TTL_DAYS,
};
use crate::consts::reasons;
use crate::crypto::{
	dummy_verify, generate_numeric_code, generate_token, hash_password, hash_token, verify_password,
};
use crate::models::{AuthSource, PasswordResetToken, Role, Session, User};
use crate::repo::{InviteRepo, PasswordResetRepo, SessionRepo, UserRepo};
use crate::services::{
	build_external_user, build_local_user, ensure_username_available, new_id, normalize_email,
	require_user, validate_password,
};
use crate::traits::{AuthProvider, ExternalIdentity, Mailer};
use crate::{Error, Result};

pub struct Authenticated {
	pub user: User,
	pub session_token: String,
	pub session_id: String,
}

pub struct AuthService {
	users: Arc<dyn UserRepo>,
	sessions: Arc<dyn SessionRepo>,
	invites: Arc<dyn InviteRepo>,
	provider: Option<Arc<dyn AuthProvider>>,
	reset_tokens: Arc<dyn PasswordResetRepo>,
	mailer: Arc<dyn Mailer>,
}

impl AuthService {
	#[allow(clippy::too_many_arguments)]
	pub fn new(
		users: Arc<dyn UserRepo>,
		sessions: Arc<dyn SessionRepo>,
		invites: Arc<dyn InviteRepo>,
		provider: Option<Arc<dyn AuthProvider>>,
		reset_tokens: Arc<dyn PasswordResetRepo>,
		mailer: Arc<dyn Mailer>,
	) -> Self {
		Self {
			users,
			sessions,
			invites,
			provider,
			reset_tokens,
			mailer,
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
		if !self.users.create_initial_admin(&user).await? {
			return Err(Error::Conflict(reasons::SETUP_COMPLETED.to_string()));
		}

		self.issue(user).await
	}

	pub async fn login(&self, username: &str, password: &str) -> Result<Authenticated> {
		if let Some(user) = self.users.find_by_username(username).await? {
			if !user.enabled {
				return Err(Error::Forbidden);
			}
			if user.auth_source == AuthSource::Local {
				let hash = user.password_hash.as_deref().ok_or(Error::Unauthorized)?;
				if !verify_password(password, hash)? {
					return Err(Error::Unauthorized);
				}
				return self.issue(user).await;
			}
			if self
				.authenticate_external(username, password)
				.await?
				.is_none()
			{
				return Err(Error::Unauthorized);
			}
			return self.issue(user).await;
		}

		let Some(identity) = self.authenticate_external(username, password).await? else {
			dummy_verify(password);
			return Err(Error::Unauthorized);
		};
		let user = self.provision_external(identity).await?;
		if !user.enabled {
			return Err(Error::Forbidden);
		}
		self.issue(user).await
	}

	async fn authenticate_external(
		&self,
		username: &str,
		password: &str,
	) -> Result<Option<ExternalIdentity>> {
		match &self.provider {
			Some(provider) => provider.authenticate(username, password).await,
			None => Ok(None),
		}
	}

	async fn provision_external(&self, identity: ExternalIdentity) -> Result<User> {
		if let Some(existing) = self.users.find_by_username(&identity.username).await? {
			if existing.auth_source == AuthSource::Local {
				return Err(Error::Unauthorized);
			}
			return Ok(existing);
		}

		let user = build_external_user(identity, Role::User);
		self.users.create(&user).await?;
		Ok(user)
	}

	async fn issue(&self, user: User) -> Result<Authenticated> {
		let token = generate_token();
		let now = Utc::now();
		let session = Session {
			id: new_id(),
			user_id: user.id.clone(),
			token_hash: hash_token(&token),
			created_at: now,
			expires_at: now + Duration::days(SESSION_TTL_DAYS),
			last_seen_at: None,
			user_agent: None,
		};
		self.sessions.create(&session).await?;
		Ok(Authenticated {
			user,
			session_token: token,
			session_id: session.id,
		})
	}

	pub async fn record_user_agent(
		&self,
		session_id: &str,
		user_agent: Option<&str>,
	) -> Result<()> {
		if let Some(user_agent) = user_agent {
			self.sessions.set_user_agent(session_id, user_agent).await?;
		}
		Ok(())
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

	pub async fn change_password(
		&self,
		user_id: &str,
		current: &str,
		new: &str,
	) -> Result<Authenticated> {
		let mut user = require_user(self.users.as_ref(), user_id).await?;

		if user.auth_source != AuthSource::Local {
			return Err(Error::Validation(reasons::PASSWORD_NOT_LOCAL.to_string()));
		}

		let hash = user.password_hash.as_deref().ok_or(Error::Unauthorized)?;
		if !verify_password(current, hash)? {
			return Err(Error::Unauthorized);
		}

		validate_password(new)?;
		user.password_hash = Some(hash_password(new)?);
		user.updated_at = Utc::now();
		self.users.update(&user).await?;

		self.sessions.delete_for_user(user_id).await?;
		self.issue(user).await
	}

	pub async fn logout(&self, token: &str) -> Result<()> {
		if let Some(session) = self.sessions.find_by_token_hash(&hash_token(token)).await? {
			self.sessions.delete(&session.id).await?;
		}
		Ok(())
	}

	pub async fn list_sessions(&self, user_id: &str) -> Result<Vec<Session>> {
		self.sessions.list_for_user(user_id).await
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

	pub async fn request_password_reset(
		&self,
		email: &str,
		accept_language: Option<&str>,
	) -> Result<()> {
		let Ok(Some(email)) = normalize_email(Some(email.to_string())) else {
			return Ok(());
		};
		let Some(user) = self.users.find_by_email(&email).await? else {
			return Ok(());
		};
		if user.auth_source != AuthSource::Local || !user.enabled {
			return Ok(());
		}

		let now = Utc::now();
		if let Some(existing) = self.reset_tokens.find_for_user(&user.id).await? {
			if now - existing.created_at < Duration::seconds(PASSWORD_RESET_COOLDOWN_SECONDS) {
				return Ok(());
			}
			self.reset_tokens.delete_for_user(&user.id).await?;
		}

		let code = generate_numeric_code(PASSWORD_RESET_CODE_DIGITS);
		self.reset_tokens
			.create(&PasswordResetToken {
				id: new_id(),
				user_id: user.id,
				code_hash: hash_token(&code),
				attempts: 0,
				created_at: now,
				expires_at: now + Duration::minutes(PASSWORD_RESET_TTL_MINUTES),
			})
			.await?;

		let locale = lunu_i18n::negotiate(accept_language, user.locale.as_deref());
		let subject = lunu_i18n::t(&locale, "email-password-reset-subject");
		let body = lunu_i18n::t_vars(
			&locale,
			"email-password-reset-body",
			&[
				("code", &code),
				("minutes", &PASSWORD_RESET_TTL_MINUTES.to_string()),
			],
		);
		let _ = self.mailer.send(&email, &subject, &body).await;
		Ok(())
	}

	pub async fn reset_password(&self, email: &str, code: &str, new_password: &str) -> Result<()> {
		let invalid = || Error::Validation(reasons::RESET_TOKEN_INVALID.to_string());

		let Ok(Some(email)) = normalize_email(Some(email.to_string())) else {
			return Err(invalid());
		};
		let Some(mut user) = self.users.find_by_email(&email).await? else {
			return Err(invalid());
		};
		let Some(record) = self.reset_tokens.find_for_user(&user.id).await? else {
			return Err(invalid());
		};
		if record.is_expired(Utc::now()) {
			self.reset_tokens.delete(&record.id).await?;
			return Err(invalid());
		}
		if record.code_hash != hash_token(code) {
			if record.attempts + 1 >= PASSWORD_RESET_MAX_ATTEMPTS {
				self.reset_tokens.delete(&record.id).await?;
			} else {
				self.reset_tokens.increment_attempts(&record.id).await?;
			}
			return Err(invalid());
		}

		if user.auth_source != AuthSource::Local {
			return Err(Error::Validation(reasons::PASSWORD_NOT_LOCAL.to_string()));
		}
		validate_password(new_password)?;

		user.password_hash = Some(hash_password(new_password)?);
		user.updated_at = Utc::now();
		self.users.update(&user).await?;

		self.reset_tokens.delete(&record.id).await?;
		self.sessions.delete_for_user(&user.id).await?;
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

		if invite.is_expired(Utc::now()) {
			return Err(Error::Validation(reasons::INVITE_UNUSABLE.to_string()));
		}

		ensure_username_available(self.users.as_ref(), username).await?;

		if !self.invites.redeem(&invite.id).await? {
			return Err(Error::Validation(reasons::INVITE_UNUSABLE.to_string()));
		}

		let user = build_local_user(username, password, invite.email.clone(), invite.role)?;
		self.users.create(&user).await?;

		self.issue(user).await
	}
}
