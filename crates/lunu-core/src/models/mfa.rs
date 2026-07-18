use std::str::FromStr;

use chrono::{DateTime, Utc};

use crate::consts::reasons;
use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MfaMethod {
	Totp,
	Email,
}

impl MfaMethod {
	pub fn as_str(&self) -> &'static str {
		match self {
			MfaMethod::Totp => "totp",
			MfaMethod::Email => "email",
		}
	}
}

impl FromStr for MfaMethod {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		match value {
			"totp" => Ok(MfaMethod::Totp),
			"email" => Ok(MfaMethod::Email),
			_ => Err(Error::Validation(reasons::MFA_METHOD_UNKNOWN.to_string())),
		}
	}
}

#[derive(Debug, Clone)]
pub struct UserMfa {
	pub user_id: String,
	pub method: MfaMethod,
	pub secret: Option<String>,
	pub confirmed: bool,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}
