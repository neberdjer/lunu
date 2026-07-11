use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use lunu_core::Error;
use serde::Serialize;

#[derive(Debug)]
pub struct ApiError(Error);

impl From<Error> for ApiError {
	fn from(error: Error) -> Self {
		Self(error)
	}
}

impl std::fmt::Display for ApiError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

#[derive(Serialize)]
struct ErrorBody {
	error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
	code: &'static str,
	message: String,
}

impl ResponseError for ApiError {
	fn status_code(&self) -> StatusCode {
		match &self.0 {
			Error::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
			Error::Unauthorized => StatusCode::UNAUTHORIZED,
			Error::Forbidden => StatusCode::FORBIDDEN,
			Error::RateLimited => StatusCode::TOO_MANY_REQUESTS,
			Error::NotFound(_) => StatusCode::NOT_FOUND,
			Error::Conflict(_) => StatusCode::CONFLICT,
			Error::Config(_) | Error::Database(_) | Error::Integration(_) | Error::Internal(_) => {
				StatusCode::INTERNAL_SERVER_ERROR
			}
		}
	}

	fn error_response(&self) -> HttpResponse {
		let status = self.status_code();
		if status.is_server_error() {
			tracing::error!(error = %self.0, "request failed");
		}

		let code = self.0.code();
		let message = lunu_i18n::error_message(&lunu_i18n::default_locale(), code, self.0.detail());

		HttpResponse::build(status).json(ErrorBody {
			error: ErrorDetail { code, message },
		})
	}
}

#[cfg(test)]
mod tests {
	use lunu_core::Error;

	fn all_variants() -> Vec<Error> {
		vec![
			Error::Config("x".to_string()),
			Error::Database("x".to_string()),
			Error::NotFound("x".to_string()),
			Error::Validation("x".to_string()),
			Error::Unauthorized,
			Error::Forbidden,
			Error::RateLimited,
			Error::Conflict("x".to_string()),
			Error::Integration("x".to_string()),
			Error::Internal("x".to_string()),
		]
	}

	#[test]
	fn every_error_code_has_a_catalog_message() {
		let locale = lunu_i18n::default_locale();
		for error in all_variants() {
			let code = error.code();
			let message = lunu_i18n::error_message(&locale, code, None);
			assert!(
				!message.starts_with("error-"),
				"no catalog message for error code '{code}'"
			);
		}
	}

	#[test]
	fn every_reason_key_has_a_catalog_message() {
		let locale = lunu_i18n::default_locale();
		for reason in lunu_core::consts::reasons::ALL {
			let key = format!("error-{reason}");
			let message = lunu_i18n::t(&locale, &key);
			assert_ne!(message, key, "no catalog message for reason key '{reason}'");
		}
	}
}
