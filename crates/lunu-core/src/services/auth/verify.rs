use chrono::{Duration, Utc};

use super::AuthService;
use crate::consts::auth::{
	EMAIL_VERIFICATION_CODE_DIGITS, EMAIL_VERIFICATION_COOLDOWN_SECONDS,
	EMAIL_VERIFICATION_MAX_ATTEMPTS, EMAIL_VERIFICATION_TTL_MINUTES,
};
use crate::consts::reasons;
use crate::consts::settings::REQUIRE_EMAIL_VERIFICATION;
use crate::crypto::{constant_time_eq, generate_numeric_code, hash_token};
use crate::email;
use crate::models::{AuthSource, EmailVerificationToken, User};
use crate::services::{new_id, nonempty, normalize_email};
use crate::{Error, Result};

impl AuthService {
	pub async fn verification_required(&self) -> Result<bool> {
		self.settings.toggle(REQUIRE_EMAIL_VERIFICATION).await
	}

	pub(super) async fn verification_pending(&self, user: &User) -> Result<bool> {
		Ok(user.auth_source == AuthSource::Local
			&& user.email.is_some()
			&& !user.email_verified
			&& self.verification_required().await?)
	}

	pub async fn notify_account_created(
		&self,
		user: &User,
		accept_language: Option<&str>,
	) -> Result<bool> {
		self.send_welcome(user, accept_language).await;

		let pending = self.verification_pending(user).await?;
		if pending {
			self.issue_verification(user, accept_language).await?;
		}
		Ok(pending)
	}

	pub async fn resend_verification(
		&self,
		email: &str,
		accept_language: Option<&str>,
	) -> Result<()> {
		if !self.verification_required().await? {
			return Ok(());
		}
		let Ok(Some(email)) = normalize_email(Some(email.to_string())) else {
			return Ok(());
		};
		let Some(user) = self.users.find_by_email(&email).await? else {
			return Ok(());
		};
		if user.auth_source != AuthSource::Local || !user.enabled || user.email_verified {
			return Ok(());
		}
		self.issue_verification(&user, accept_language).await
	}

	pub async fn verify_email(&self, email: &str, code: &str) -> Result<()> {
		let invalid = || Error::Validation(reasons::VERIFICATION_INVALID.to_string());

		let Ok(Some(email)) = normalize_email(Some(email.to_string())) else {
			return Err(invalid());
		};
		let Some(user) = self.users.find_by_email(&email).await? else {
			return Err(invalid());
		};
		if user.email_verified {
			return Err(invalid());
		}
		let Some(record) = self.email_verifications.find_for_user(&user.id).await? else {
			return Err(invalid());
		};
		if record.is_expired(Utc::now()) {
			self.email_verifications.delete(&record.id).await?;
			return Err(invalid());
		}
		if !self
			.email_verifications
			.claim_attempt(&record.id, EMAIL_VERIFICATION_MAX_ATTEMPTS)
			.await?
		{
			return Err(invalid());
		}
		if !constant_time_eq(&record.code_hash, &hash_token(code)) {
			return Err(invalid());
		}

		self.users.mark_email_verified(&user.id).await?;
		self.email_verifications.delete(&record.id).await?;
		Ok(())
	}

	async fn issue_verification(&self, user: &User, accept_language: Option<&str>) -> Result<()> {
		let Some(address) = nonempty(user.email.clone()) else {
			return Ok(());
		};

		let now = Utc::now();
		if let Some(existing) = self.email_verifications.find_for_user(&user.id).await? {
			if now - existing.created_at < Duration::seconds(EMAIL_VERIFICATION_COOLDOWN_SECONDS) {
				return Ok(());
			}
			self.email_verifications.delete_for_user(&user.id).await?;
		}

		let code = generate_numeric_code(EMAIL_VERIFICATION_CODE_DIGITS);
		self.email_verifications
			.create(&EmailVerificationToken {
				id: new_id(),
				user_id: user.id.clone(),
				code_hash: hash_token(&code),
				attempts: 0,
				created_at: now,
				expires_at: now + Duration::minutes(EMAIL_VERIFICATION_TTL_MINUTES),
			})
			.await?;

		let locale = lunu_i18n::negotiate(accept_language, user.locale.as_deref());
		let rendered = email::verification(&locale, &code, EMAIL_VERIFICATION_TTL_MINUTES);
		let _ = self.mailer.send(&address, &rendered).await;
		Ok(())
	}

	async fn send_welcome(&self, user: &User, accept_language: Option<&str>) {
		let Some(address) = nonempty(user.email.clone()) else {
			return;
		};
		let locale = lunu_i18n::negotiate(accept_language, user.locale.as_deref());
		let rendered = email::welcome(&locale, &user.username);
		let _ = self.mailer.send(&address, &rendered).await;
	}
}
