use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};

use crate::consts::reasons;
use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
	Admin,
	User,
}

impl Role {
	pub fn as_str(&self) -> &'static str {
		match self {
			Role::Admin => "admin",
			Role::User => "user",
		}
	}

	pub fn is_admin(&self) -> bool {
		matches!(self, Role::Admin)
	}
}

impl fmt::Display for Role {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

impl FromStr for Role {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		match value {
			"admin" => Ok(Role::Admin),
			"user" => Ok(Role::User),
			_ => Err(Error::Validation(reasons::ROLE_UNKNOWN.to_string())),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthSource {
	Local,
	Abs,
	Oidc,
	Proxy,
}

impl AuthSource {
	pub fn as_str(&self) -> &'static str {
		match self {
			AuthSource::Local => "local",
			AuthSource::Oidc => "oidc",
			AuthSource::Proxy => "proxy",
			AuthSource::Abs => "abs",
		}
	}
}

impl fmt::Display for AuthSource {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

impl FromStr for AuthSource {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		match value {
			"local" => Ok(AuthSource::Local),
			"abs" => Ok(AuthSource::Abs),
			"oidc" => Ok(AuthSource::Oidc),
			"proxy" => Ok(AuthSource::Proxy),
			_ => Err(Error::Validation(reasons::AUTH_SOURCE_UNKNOWN.to_string())),
		}
	}
}

#[derive(Clone, PartialEq, Eq)]
pub struct User {
	pub id: String,
	pub username: String,
	pub email: Option<String>,
	pub display_name: Option<String>,
	pub locale: Option<String>,
	pub password_hash: Option<String>,
	pub role: Role,
	pub auth_source: AuthSource,
	pub oidc_subject: Option<String>,
	pub enabled: bool,
	pub email_verified: bool,
	pub notify_email: bool,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

impl std::fmt::Debug for User {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("User")
			.field("id", &self.id)
			.field("username", &self.username)
			.field("email", &self.email.as_ref().map(|_| "<redacted>"))
			.field("display_name", &self.display_name)
			.field("locale", &self.locale)
			.field(
				"password_hash",
				&self.password_hash.as_ref().map(|_| "<redacted>"),
			)
			.field("role", &self.role)
			.field("auth_source", &self.auth_source)
			.field("oidc_subject", &self.oidc_subject)
			.field("enabled", &self.enabled)
			.field("email_verified", &self.email_verified)
			.field("notify_email", &self.notify_email)
			.field("created_at", &self.created_at)
			.field("updated_at", &self.updated_at)
			.finish()
	}
}
