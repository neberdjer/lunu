use chrono::{Duration, Utc};

use super::{PendingMfa, unix_seconds};
use crate::consts::auth::{MFA_CODE_DIGITS, MFA_MAX_ATTEMPTS, MFA_TICKET_TTL_MINUTES};
use crate::consts::reasons;
use crate::crypto::{
	constant_time_eq, generate_numeric_code, generate_token, hash_token, totp_matches,
};
use crate::email;
use crate::models::{MfaMethod, User};
use crate::services::auth::{AuthService, Authenticated, LoginOutcome, MfaChallenge};
use crate::services::nonempty;
use crate::{Error, Result};

impl AuthService {
	pub(in crate::services::auth) async fn login_challenge(
		&self,
		user: User,
	) -> Result<LoginOutcome> {
		let Some(mfa) = self
			.mfa
			.find_for_user(&user.id)
			.await?
			.filter(|mfa| mfa.confirmed)
		else {
			return Ok(LoginOutcome::Authenticated(Box::new(
				self.issue(user).await?,
			)));
		};

		let ticket = generate_token();
		let secret = match mfa.method {
			MfaMethod::Totp => Some(self.decrypt_mfa_secret(&mfa)?),
			MfaMethod::Email => None,
		};
		let code = (mfa.method == MfaMethod::Email).then(|| generate_numeric_code(MFA_CODE_DIGITS));
		if let Some(code) = &code {
			let _ = self.deliver_mfa_email(&user, code, None).await;
		}

		let mut pending = self.mfa_pending.lock().expect("mfa ticket lock");
		let deadline = Utc::now() - Duration::minutes(MFA_TICKET_TTL_MINUTES);
		pending.retain(|_, entry| entry.created_at > deadline);
		pending.insert(
			ticket.clone(),
			PendingMfa {
				user_id: user.id.clone(),
				method: mfa.method,
				secret,
				code_hash: code.as_deref().map(hash_token),
				attempts: 0,
				created_at: Utc::now(),
			},
		);
		Ok(LoginOutcome::MfaRequired(MfaChallenge {
			ticket,
			method: mfa.method,
		}))
	}

	pub async fn mfa_verify(&self, ticket: &str, code: &str) -> Result<Authenticated> {
		let user_id = self.verify_challenge(ticket, code).await?;
		let user = self
			.users
			.find_by_id(&user_id)
			.await?
			.ok_or_else(|| Error::Validation(reasons::MFA_TICKET_INVALID.to_string()))?;
		if !user.enabled {
			return Err(Error::Forbidden);
		}
		self.issue(user).await
	}

	async fn verify_challenge(&self, ticket: &str, code: &str) -> Result<String> {
		let (user_id, matched) = self.consume_pending(ticket, code)?;
		if matched {
			return Ok(user_id);
		}
		if self
			.recovery
			.consume(&user_id, &hash_token(code.trim()))
			.await?
		{
			self.mfa_pending
				.lock()
				.expect("mfa ticket lock")
				.remove(ticket);
			Ok(user_id)
		} else {
			Err(Error::Validation(reasons::MFA_CODE_INVALID.to_string()))
		}
	}

	pub(super) fn consume_pending(&self, ticket: &str, code: &str) -> Result<(String, bool)> {
		let mut pending = self.mfa_pending.lock().expect("mfa ticket lock");
		take_verified(&mut pending, ticket, code)
	}

	pub(super) fn consume_email_pending(&self, user_id: &str, code: &str) -> Result<()> {
		let mut pending = self.mfa_pending.lock().expect("mfa ticket lock");
		let ticket = pending
			.iter()
			.find(|(_, entry)| entry.user_id == user_id && entry.method == MfaMethod::Email)
			.map(|(ticket, _)| ticket.clone())
			.ok_or_else(|| Error::Validation(reasons::MFA_TICKET_INVALID.to_string()))?;
		let (_, matched) = take_verified(&mut pending, &ticket, code)?;
		if matched {
			Ok(())
		} else {
			Err(Error::Validation(reasons::MFA_CODE_INVALID.to_string()))
		}
	}

	pub(super) async fn mfa_confirmed(&self, user_id: &str) -> Result<bool> {
		Ok(self
			.mfa
			.find_for_user(user_id)
			.await?
			.is_some_and(|mfa| mfa.confirmed))
	}

	pub(super) async fn issue_mfa_email(
		&self,
		user: &User,
		accept_language: Option<&str>,
	) -> Result<()> {
		let code = generate_numeric_code(MFA_CODE_DIGITS);
		{
			let ticket = generate_token();
			let mut pending = self.mfa_pending.lock().expect("mfa ticket lock");
			let deadline = Utc::now() - Duration::minutes(MFA_TICKET_TTL_MINUTES);
			pending.retain(|_, entry| {
				entry.created_at > deadline
					&& !(entry.user_id == user.id && entry.method == MfaMethod::Email)
			});
			pending.insert(
				ticket,
				PendingMfa {
					user_id: user.id.clone(),
					method: MfaMethod::Email,
					secret: None,
					code_hash: Some(hash_token(&code)),
					attempts: 0,
					created_at: Utc::now(),
				},
			);
		}
		self.deliver_mfa_email(user, &code, accept_language).await
	}

	async fn deliver_mfa_email(
		&self,
		user: &User,
		code: &str,
		accept_language: Option<&str>,
	) -> Result<()> {
		let Some(address) = nonempty(user.email.clone()) else {
			return Ok(());
		};
		let locale = lunu_i18n::negotiate(accept_language, user.locale.as_deref());
		let rendered = email::mfa_code(&locale, code, MFA_TICKET_TTL_MINUTES);
		let _ = self
			.mailer
			.send(&address, &rendered.subject, &rendered.html)
			.await;
		Ok(())
	}
}

fn take_verified(
	pending: &mut std::collections::HashMap<String, PendingMfa>,
	ticket: &str,
	code: &str,
) -> Result<(String, bool)> {
	let invalid = || Error::Validation(reasons::MFA_TICKET_INVALID.to_string());
	let entry = pending.get_mut(ticket).ok_or_else(invalid)?;
	if entry.created_at < Utc::now() - Duration::minutes(MFA_TICKET_TTL_MINUTES)
		|| entry.attempts >= MFA_MAX_ATTEMPTS
	{
		pending.remove(ticket);
		return Err(invalid());
	}

	let ok = match entry.method {
		MfaMethod::Totp => entry
			.secret
			.as_deref()
			.is_some_and(|secret| totp_matches(secret, unix_seconds(Utc::now()), code)),
		MfaMethod::Email => entry
			.code_hash
			.as_deref()
			.is_some_and(|hash| constant_time_eq(hash, &hash_token(code.trim()))),
	};
	if !ok {
		entry.attempts += 1;
		return Ok((entry.user_id.clone(), false));
	}

	let user_id = entry.user_id.clone();
	pending.remove(ticket);
	Ok((user_id, true))
}
