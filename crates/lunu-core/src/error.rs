use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
	#[error("configuration error: {0}")]
	Config(String),

	#[error("database error: {0}")]
	Database(String),

	#[error("not found: {0}")]
	NotFound(String),

	#[error("bad request: {0}")]
	BadRequest(String),

	#[error("validation error: {0}")]
	Validation(String),

	#[error("unauthorized")]
	Unauthorized,

	#[error("forbidden")]
	Forbidden,

	#[error("too many requests")]
	RateLimited,

	#[error("conflict: {0}")]
	Conflict(String),

	#[error("integration error: {0}")]
	Integration(String),

	#[error("internal error: {0}")]
	Internal(String),
}

impl Error {
	pub fn code(&self) -> &'static str {
		match self {
			Error::Config(_) => "config",
			Error::Database(_) => "database",
			Error::NotFound(_) => "not_found",
			Error::BadRequest(_) => "bad_request",
			Error::Validation(_) => "validation",
			Error::Unauthorized => "unauthorized",
			Error::Forbidden => "forbidden",
			Error::RateLimited => "rate_limited",
			Error::Conflict(_) => "conflict",
			Error::Integration(_) => "integration",
			Error::Internal(_) => "internal",
		}
	}

	pub fn detail(&self) -> Option<&str> {
		match self {
			Error::Config(detail)
			| Error::Database(detail)
			| Error::NotFound(detail)
			| Error::BadRequest(detail)
			| Error::Validation(detail)
			| Error::Conflict(detail)
			| Error::Integration(detail)
			| Error::Internal(detail) => Some(detail.as_str()),
			Error::Unauthorized | Error::Forbidden | Error::RateLimited => None,
		}
	}
}

impl From<serde_json::Error> for Error {
	fn from(error: serde_json::Error) -> Self {
		Error::Internal(error.to_string())
	}
}

pub type Result<T> = std::result::Result<T, Error>;
