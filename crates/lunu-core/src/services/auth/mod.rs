use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::consts::auth::SESSION_TTL_DAYS;
use crate::consts::reasons;
use crate::crypto::Encryptor;
use crate::crypto::{dummy_verify, generate_token, hash_password, hash_token, verify_password};
use crate::models::{AuthSource, Role, Session, User};
use crate::repo::{EmailVerificationRepo, InviteRepo, PasswordResetRepo, SessionRepo, UserRepo};
use crate::repo::{MfaRecoveryCodeRepo, UserMfaRepo};
use crate::services::{
	SettingsService, build_external_user, build_local_user, ensure_username_available, new_id,
	require_user, validate_password,
};
use crate::traits::{AuthProvider, ExternalIdentity, Mailer, OidcFlow};
use crate::{Error, Result};

mod device;
mod mfa;
mod oidc;
pub use mfa::{MfaChallenge, MfaEnrollment, MfaStatus};
pub use oidc::OidcStart;
mod reset;
mod session;
mod verify;

pub struct Authenticated {
	pub user: User,
	pub session_token: String,
	pub session_id: String,
}

pub enum Registration {
	Active(Box<Authenticated>),
	PendingVerification,
}

pub enum LoginOutcome {
	Authenticated(Box<Authenticated>),
	MfaRequired(MfaChallenge),
}

pub struct AuthService {
	users: Arc<dyn UserRepo>,
	sessions: Arc<dyn SessionRepo>,
	invites: Arc<dyn InviteRepo>,
	provider: Option<Arc<dyn AuthProvider>>,
	oidc: Option<Arc<dyn OidcFlow>>,
	oidc_pending: std::sync::Mutex<std::collections::HashMap<String, oidc::PendingLogin>>,
	mfa: Arc<dyn UserMfaRepo>,
	recovery: Arc<dyn MfaRecoveryCodeRepo>,
	mfa_pending: std::sync::Mutex<std::collections::HashMap<String, mfa::PendingMfa>>,
	encryptor: Encryptor,
	reset_tokens: Arc<dyn PasswordResetRepo>,
	email_verifications: Arc<dyn EmailVerificationRepo>,
	settings: Arc<SettingsService>,
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
		email_verifications: Arc<dyn EmailVerificationRepo>,
		mfa: Arc<dyn UserMfaRepo>,
		recovery: Arc<dyn MfaRecoveryCodeRepo>,
		encryptor: Encryptor,
		settings: Arc<SettingsService>,
		mailer: Arc<dyn Mailer>,
	) -> Self {
		Self {
			users,
			sessions,
			invites,
			provider,
			oidc: None,
			oidc_pending: std::sync::Mutex::new(std::collections::HashMap::new()),
			mfa,
			recovery,
			mfa_pending: std::sync::Mutex::new(std::collections::HashMap::new()),
			encryptor,
			reset_tokens,
			email_verifications,
			settings,
			mailer,
		}
	}

	pub fn with_oidc(mut self, flow: Arc<dyn OidcFlow>) -> Self {
		self.oidc = Some(flow);
		self
	}

	pub(super) async fn setting(&self, key: &str) -> Result<Option<String>> {
		Ok(self
			.settings
			.get(key)
			.await?
			.map(|value| value.trim().to_string())
			.filter(|value| !value.is_empty()))
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

		let mut user = build_local_user(username, password, email, Role::Admin)?;
		user.email_verified = true;
		if !self.users.create_initial_admin(&user).await? {
			return Err(Error::Conflict(reasons::SETUP_COMPLETED.to_string()));
		}

		self.issue(user).await
	}

	pub async fn login(&self, username: &str, password: &str) -> Result<LoginOutcome> {
		if let Some(user) = self.users.find_by_username(username).await? {
			if user.auth_source == AuthSource::Local {
				let hash = user.password_hash.as_deref().ok_or(Error::Unauthorized)?;
				if !verify_password(password, hash)? {
					return Err(Error::Unauthorized);
				}
				if !user.enabled {
					return Err(Error::Forbidden);
				}
				if self.verification_pending(&user).await? {
					return Err(Error::Validation(reasons::EMAIL_NOT_VERIFIED.to_string()));
				}
				return self.login_challenge(user).await;
			}
			if self
				.authenticate_external(username, password)
				.await?
				.is_none()
			{
				return Err(Error::Unauthorized);
			}
			if !user.enabled {
				return Err(Error::Forbidden);
			}
			return self.login_challenge(user).await;
		}

		let Some(identity) = self.authenticate_external(username, password).await? else {
			dummy_verify(password);
			return Err(Error::Unauthorized);
		};
		let user = self.provision_external(identity).await?;
		if !user.enabled {
			return Err(Error::Forbidden);
		}
		self.login_challenge(user).await
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

		if self.verification_pending(&user).await? {
			return Err(Error::Validation(reasons::EMAIL_NOT_VERIFIED.to_string()));
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

	pub async fn register_with_invite(
		&self,
		code: &str,
		username: &str,
		password: &str,
		accept_language: Option<&str>,
	) -> Result<Registration> {
		let invite = self
			.invites
			.find_by_code_hash(&hash_token(code))
			.await?
			.ok_or_else(|| Error::Validation(reasons::INVITE_INVALID.to_string()))?;

		if invite.is_expired(Utc::now()) {
			return Err(Error::Validation(reasons::INVITE_UNUSABLE.to_string()));
		}

		ensure_username_available(self.users.as_ref(), username).await?;

		let user = build_local_user(username, password, invite.email.clone(), invite.role)?;

		if !self.invites.redeem(&invite.id).await? {
			return Err(Error::Validation(reasons::INVITE_UNUSABLE.to_string()));
		}

		self.users.create(&user).await?;

		let pending_verification = self.notify_account_created(&user, accept_language).await?;
		if pending_verification {
			return Ok(Registration::PendingVerification);
		}

		Ok(Registration::Active(Box::new(self.issue(user).await?)))
	}
}
