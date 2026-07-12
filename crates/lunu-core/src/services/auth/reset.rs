use chrono::{Duration, Utc};

use super::AuthService;
use crate::consts::auth::{
	PASSWORD_RESET_CODE_DIGITS, PASSWORD_RESET_COOLDOWN_SECONDS, PASSWORD_RESET_MAX_ATTEMPTS,
	PASSWORD_RESET_TTL_MINUTES,
};
use crate::consts::reasons;
use crate::crypto::{generate_numeric_code, hash_password, hash_token};
use crate::models::{AuthSource, PasswordResetToken};
use crate::services::{new_id, normalize_email, validate_password};
use crate::{Error, Result};

impl AuthService {
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
}
