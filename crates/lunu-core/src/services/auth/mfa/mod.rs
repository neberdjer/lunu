use chrono::{DateTime, Utc};

use super::AuthService;
use crate::consts::auth::{MFA_RECOVERY_CODE_COUNT, TOTP_DIGITS, TOTP_ISSUER, TOTP_STEP_SECONDS};
use crate::consts::reasons;
use crate::crypto::{generate_recovery_code, generate_totp_secret, hash_token, totp_matches};
use crate::models::{MfaMethod, MfaRecoveryCode, User, UserMfa};
use crate::services::{new_id, nonempty};
use crate::{Error, Result};

mod login;

pub(super) struct PendingMfa {
	user_id: String,
	method: MfaMethod,
	secret: Option<String>,
	code_hash: Option<String>,
	attempts: i64,
	created_at: DateTime<Utc>,
}

pub struct MfaChallenge {
	pub ticket: String,
	pub method: MfaMethod,
}

pub struct MfaEnrollment {
	pub method: MfaMethod,
	pub secret: Option<String>,
	pub otpauth_uri: Option<String>,
}

pub struct MfaStatus {
	pub enabled: bool,
	pub method: Option<MfaMethod>,
	pub recovery_codes_remaining: Option<i64>,
}

pub(super) fn unix_seconds(now: DateTime<Utc>) -> u64 {
	now.timestamp().max(0) as u64
}

impl AuthService {
	pub async fn mfa_status(&self, user: &User) -> Result<MfaStatus> {
		let enrolled = self
			.mfa
			.find_for_user(&user.id)
			.await?
			.filter(|mfa| mfa.confirmed);
		let recovery_codes_remaining = match &enrolled {
			Some(_) => Some(self.recovery.count_unused(&user.id).await?),
			None => None,
		};
		Ok(MfaStatus {
			enabled: enrolled.is_some(),
			method: enrolled.map(|mfa| mfa.method),
			recovery_codes_remaining,
		})
	}

	pub async fn mfa_begin_enrollment(
		&self,
		user: &User,
		method: MfaMethod,
	) -> Result<MfaEnrollment> {
		if self.mfa_confirmed(&user.id).await? {
			return Err(Error::Conflict(reasons::MFA_ALREADY_ENABLED.to_string()));
		}
		if method == MfaMethod::Email && nonempty(user.email.clone()).is_none() {
			return Err(Error::Validation(reasons::MFA_EMAIL_REQUIRED.to_string()));
		}

		let now = Utc::now();
		let secret = (method == MfaMethod::Totp).then(generate_totp_secret);
		let stored_secret = secret
			.as_deref()
			.map(|s| self.encryptor.encrypt(s))
			.transpose()?;
		self.mfa
			.upsert(&UserMfa {
				user_id: user.id.clone(),
				method,
				secret: stored_secret,
				confirmed: false,
				created_at: now,
				updated_at: now,
			})
			.await?;

		let otpauth_uri = secret
			.as_deref()
			.map(|secret| totp_uri(&user.username, secret));
		Ok(MfaEnrollment {
			method,
			secret,
			otpauth_uri,
		})
	}

	pub async fn mfa_confirm_enrollment(&self, user: &User, code: &str) -> Result<Vec<String>> {
		let mfa = self
			.mfa
			.find_for_user(&user.id)
			.await?
			.ok_or_else(|| Error::Validation(reasons::MFA_NOT_ENROLLED.to_string()))?;
		if mfa.confirmed {
			return Err(Error::Conflict(reasons::MFA_ALREADY_ENABLED.to_string()));
		}

		match mfa.method {
			MfaMethod::Totp => {
				let secret = self.decrypt_mfa_secret(&mfa)?;
				if !totp_matches(&secret, unix_seconds(Utc::now()), code) {
					return Err(Error::Validation(reasons::MFA_CODE_INVALID.to_string()));
				}
			}
			MfaMethod::Email => self.consume_email_pending(&user.id, code)?,
		}

		self.mfa
			.upsert(&UserMfa {
				confirmed: true,
				updated_at: Utc::now(),
				..mfa
			})
			.await?;
		self.issue_recovery_codes(&user.id).await
	}

	pub async fn mfa_regenerate_recovery_codes(&self, user: &User) -> Result<Vec<String>> {
		if !self.mfa_confirmed(&user.id).await? {
			return Err(Error::Validation(reasons::MFA_NOT_ENROLLED.to_string()));
		}
		self.issue_recovery_codes(&user.id).await
	}

	pub async fn mfa_admin_reset(&self, user_id: &str) -> Result<()> {
		self.users
			.find_by_id(user_id)
			.await?
			.ok_or_else(|| Error::NotFound(format!("user {user_id}")))?;
		self.purge_mfa(user_id).await
	}

	async fn purge_mfa(&self, user_id: &str) -> Result<()> {
		self.recovery.delete_for_user(user_id).await?;
		self.mfa.delete_for_user(user_id).await
	}

	async fn issue_recovery_codes(&self, user_id: &str) -> Result<Vec<String>> {
		let now = Utc::now();
		let codes: Vec<String> = (0..MFA_RECOVERY_CODE_COUNT)
			.map(|_| generate_recovery_code())
			.collect();
		let rows: Vec<MfaRecoveryCode> = codes
			.iter()
			.map(|code| MfaRecoveryCode {
				id: new_id(),
				user_id: user_id.to_string(),
				code_hash: hash_token(code),
				used_at: None,
				created_at: now,
			})
			.collect();
		self.recovery.replace_for_user(user_id, &rows).await?;
		Ok(codes)
	}

	pub async fn mfa_send_enrollment_code(
		&self,
		user: &User,
		accept_language: Option<&str>,
	) -> Result<()> {
		self.mfa
			.find_for_user(&user.id)
			.await?
			.filter(|mfa| mfa.method == MfaMethod::Email && !mfa.confirmed)
			.ok_or_else(|| Error::Validation(reasons::MFA_NOT_ENROLLED.to_string()))?;
		self.issue_mfa_email(user, accept_language).await
	}

	pub async fn mfa_disable(&self, user: &User) -> Result<()> {
		self.purge_mfa(&user.id).await
	}

	pub(super) fn decrypt_mfa_secret(&self, mfa: &UserMfa) -> Result<String> {
		let secret = mfa
			.secret
			.as_deref()
			.ok_or_else(|| Error::Validation(reasons::MFA_NOT_ENROLLED.to_string()))?;
		self.encryptor.decrypt(secret)
	}
}
fn totp_uri(username: &str, secret: &str) -> String {
	let label = format!("{TOTP_ISSUER}:{username}");
	let encoded_label = url_encode(&label);
	format!(
		"otpauth://totp/{encoded_label}?secret={secret}&issuer={TOTP_ISSUER}&digits={TOTP_DIGITS}&period={TOTP_STEP_SECONDS}"
	)
}

fn url_encode(value: &str) -> String {
	let mut out = String::with_capacity(value.len());
	for byte in value.bytes() {
		match byte {
			b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
				out.push(byte as char)
			}
			_ => out.push_str(&format!("%{byte:02X}")),
		}
	}
	out
}
