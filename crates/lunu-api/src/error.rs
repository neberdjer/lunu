use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use lunu_core::Error;
use lunu_core::consts::reasons;
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
	#[serde(skip_serializing_if = "Option::is_none")]
	reason: Option<String>,
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
			Error::BadRequest(_) => StatusCode::BAD_REQUEST,
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
		let detail = self.0.detail();
		let message = lunu_i18n::error_message(&lunu_i18n::default_locale(), code, detail);
		let reason = detail
			.filter(|value| reasons::ALL.contains(value))
			.map(str::to_string);

		envelope(status, code, reason, message)
	}
}

fn envelope(
	status: StatusCode,
	code: &'static str,
	reason: Option<String>,
	message: String,
) -> HttpResponse {
	HttpResponse::build(status).json(ErrorBody {
		error: ErrorDetail {
			code,
			reason,
			message,
		},
	})
}

pub(crate) fn status_error_response(status: StatusCode) -> HttpResponse {
	if status.is_server_error() {
		tracing::error!(%status, "framework error response");
	}
	let code = status_slug(status);
	let message = lunu_i18n::error_message(&lunu_i18n::default_locale(), code, None);
	envelope(status, code, None, message)
}

fn status_slug(status: StatusCode) -> &'static str {
	match status {
		StatusCode::BAD_REQUEST => "bad_request",
		StatusCode::UNAUTHORIZED => "unauthorized",
		StatusCode::FORBIDDEN => "forbidden",
		StatusCode::NOT_FOUND => "not_found",
		StatusCode::METHOD_NOT_ALLOWED => "method_not_allowed",
		StatusCode::CONFLICT => "conflict",
		StatusCode::UNPROCESSABLE_ENTITY => "validation",
		StatusCode::TOO_MANY_REQUESTS => "rate_limited",
		_ if status.is_server_error() => "internal",
		_ => "bad_request",
	}
}

#[cfg(test)]
mod tests {
	use lunu_core::Error;

	use super::*;

	#[actix_web::test]
	async fn envelope_exposes_reason_for_registered_reason_keys() {
		let response =
			ApiError(Error::Validation("setting-invalid-url".to_string())).error_response();
		assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

		let bytes = actix_web::body::to_bytes(response.into_body())
			.await
			.unwrap();
		let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
		assert_eq!(value["error"]["code"], "validation");
		assert_eq!(value["error"]["reason"], "setting-invalid-url");
		assert_eq!(
			value["error"]["message"],
			"Enter a valid URL starting with http:// or https://."
		);
	}

	#[actix_web::test]
	async fn status_error_response_envelopes_framework_status() {
		let response = status_error_response(StatusCode::METHOD_NOT_ALLOWED);
		assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

		let bytes = actix_web::body::to_bytes(response.into_body())
			.await
			.unwrap();
		let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
		assert_eq!(value["error"]["code"], "method_not_allowed");
		assert_eq!(
			value["error"]["message"],
			"That method is not allowed on this resource."
		);
	}

	#[actix_web::test]
	async fn envelope_omits_reason_for_diagnostic_details() {
		let response = ApiError(Error::NotFound("user abc".to_string())).error_response();
		assert_eq!(response.status(), StatusCode::NOT_FOUND);

		let bytes = actix_web::body::to_bytes(response.into_body())
			.await
			.unwrap();
		let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
		assert_eq!(value["error"]["code"], "not_found");
		assert!(value["error"].get("reason").is_none());
		assert_eq!(
			value["error"]["message"],
			"The requested resource was not found."
		);
	}

	fn all_variants() -> Vec<Error> {
		vec![
			Error::Config("x".to_string()),
			Error::Database("x".to_string()),
			Error::NotFound("x".to_string()),
			Error::BadRequest("x".to_string()),
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
	fn every_status_slug_has_a_catalog_message() {
		let locale = lunu_i18n::default_locale();
		let statuses = [
			StatusCode::BAD_REQUEST,
			StatusCode::UNAUTHORIZED,
			StatusCode::FORBIDDEN,
			StatusCode::NOT_FOUND,
			StatusCode::METHOD_NOT_ALLOWED,
			StatusCode::CONFLICT,
			StatusCode::UNPROCESSABLE_ENTITY,
			StatusCode::TOO_MANY_REQUESTS,
			StatusCode::INTERNAL_SERVER_ERROR,
			StatusCode::UNSUPPORTED_MEDIA_TYPE,
		];
		for status in statuses {
			let code = status_slug(status);
			let message = lunu_i18n::error_message(&locale, code, None);
			assert!(
				!message.starts_with("error-"),
				"no catalog message for status slug '{code}'"
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
