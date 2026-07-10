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
