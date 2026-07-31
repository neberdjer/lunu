use chrono::{DateTime, Duration, Utc};

use crate::consts::auth::OIDC_STATE_TTL_MINS;
use crate::consts::reasons;
use crate::consts::settings::{BASE_URL, OIDC_ISSUER_URL};
use crate::crypto::{constant_time_eq, generate_token, hash_token, pkce_challenge};
use crate::models::{AuthSource, Role, User};
use crate::services::{
	ProvisionedUser, adopt_existing, build_provisioned_user, new_id, normalize_email,
	validate_username,
};
use crate::traits::OidcClaims;
use crate::{Error, Result};

use super::{AuthService, Authenticated};

pub(super) struct PendingLogin {
	verifier: String,
	binding: String,
	created_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct OidcStart {
	pub url: String,
	pub binding: String,
}

impl AuthService {
	pub async fn oidc_start(&self) -> Result<OidcStart> {
		let flow = self
			.oidc
			.as_ref()
			.ok_or_else(|| Error::Validation(reasons::OIDC_NOT_CONFIGURED.to_string()))?;
		let redirect_uri = self.oidc_redirect_uri().await?;

		let state = generate_token();
		let verifier = generate_token();
		let binding = generate_token();
		let challenge = pkce_challenge(&verifier);
		let url = flow
			.authorize_url(&state, &challenge, &redirect_uri)
			.await?;

		let mut pending = self.oidc_pending.lock().expect("oidc state lock");
		let deadline = Utc::now() - Duration::minutes(OIDC_STATE_TTL_MINS);
		pending.retain(|_, login| login.created_at > deadline);
		pending.insert(
			state,
			PendingLogin {
				verifier,
				binding: binding.clone(),
				created_at: Utc::now(),
			},
		);
		Ok(OidcStart { url, binding })
	}

	pub async fn oidc_callback(
		&self,
		state: &str,
		code: &str,
		binding: &str,
	) -> Result<Authenticated> {
		let flow = self
			.oidc
			.as_ref()
			.ok_or_else(|| Error::Validation(reasons::OIDC_NOT_CONFIGURED.to_string()))?;
		let redirect_uri = self.oidc_redirect_uri().await?;

		let login = self
			.oidc_pending
			.lock()
			.expect("oidc state lock")
			.remove(state)
			.filter(|login| login.created_at > Utc::now() - Duration::minutes(OIDC_STATE_TTL_MINS))
			.filter(|login| crypto_eq(&login.binding, binding))
			.ok_or_else(|| Error::Validation(reasons::OIDC_STATE_INVALID.to_string()))?;

		let claims = flow.exchange(code, &login.verifier, &redirect_uri).await?;
		let user = self.oidc_user(claims).await?;
		if !user.enabled {
			return Err(Error::Forbidden);
		}
		self.issue(user).await
	}

	async fn oidc_redirect_uri(&self) -> Result<String> {
		let base = self
			.setting(BASE_URL)
			.await?
			.ok_or_else(|| Error::Validation(reasons::OIDC_NOT_CONFIGURED.to_string()))?;
		Ok(format!(
			"{}{}/auth/oidc/callback",
			base.trim_end_matches('/'),
			crate::consts::api::API_PREFIX
		))
	}

	async fn oidc_user(&self, claims: OidcClaims) -> Result<User> {
		if claims.subject.trim().is_empty() {
			return Err(Error::Validation(reasons::OIDC_STATE_INVALID.to_string()));
		}
		let scoped_subject = self.scoped_oidc_subject(&claims.subject).await?;
		if let Some(existing) = self.users.find_by_oidc_subject(&scoped_subject).await? {
			return Ok(existing);
		}

		let wanted = validate_username(&claims.preferred_name())
			.unwrap_or_else(|_| format!("user-{}", &new_id()[..8]));
		let email = normalize_email(claims.email).unwrap_or(None);
		if let Some(address) = email.as_deref()
			&& self.users.find_by_email(address).await?.is_some()
		{
			return Err(Error::Conflict(reasons::OIDC_ACCOUNT_CONFLICT.to_string()));
		}

		let email_verified = claims.email_verified && email.is_some();
		let username = self.available_username(wanted).await?;
		let user = build_provisioned_user(
			ProvisionedUser {
				username,
				email,
				email_verified,
				display_name: claims.display_name,
				auth_source: AuthSource::Oidc,
				oidc_subject: Some(scoped_subject),
			},
			Role::User,
		);
		self.users.create(&user).await?;
		Ok(user)
	}

	async fn scoped_oidc_subject(&self, subject: &str) -> Result<String> {
		let issuer = self.setting(OIDC_ISSUER_URL).await?.unwrap_or_default();
		Ok(format!("{}\n{subject}", issuer.trim_end_matches('/')))
	}

	pub async fn proxy_user(&self, username: &str) -> Result<User> {
		if let Some(existing) = self.users.find_by_username(username).await? {
			return adopt_existing(existing, AuthSource::Proxy);
		}

		let user = build_provisioned_user(
			ProvisionedUser {
				username: username.to_string(),
				email: None,
				email_verified: false,
				display_name: None,
				auth_source: AuthSource::Proxy,
				oidc_subject: None,
			},
			Role::User,
		);
		self.users.create(&user).await?;
		Ok(user)
	}

	async fn available_username(&self, wanted: String) -> Result<String> {
		if self.users.find_by_username(&wanted).await?.is_none() {
			return Ok(wanted);
		}
		let suffix = &new_id()[..8];
		Ok(format!("{wanted}-{suffix}"))
	}
}

fn crypto_eq(known: &str, presented: &str) -> bool {
	constant_time_eq(&hash_token(known), &hash_token(presented))
}
