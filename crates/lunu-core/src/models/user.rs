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
}

impl AuthSource {
	pub fn as_str(&self) -> &'static str {
		match self {
			AuthSource::Local => "local",
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
			_ => Err(Error::Validation(reasons::AUTH_SOURCE_UNKNOWN.to_string())),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
	pub id: String,
	pub username: String,
	pub email: Option<String>,
	pub display_name: Option<String>,
	pub locale: Option<String>,
	pub password_hash: Option<String>,
	pub role: Role,
	pub auth_source: AuthSource,
	pub enabled: bool,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}
