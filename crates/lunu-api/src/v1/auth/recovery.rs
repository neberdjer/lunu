use actix_web::{HttpRequest, HttpResponse, post, web};
use serde::Deserialize;

use crate::dto::StatusResponse;
use crate::error::ApiError;
use crate::extract::accept_language;
use crate::state::AppState;

use super::enforce_auth_rate_limit;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ForgotPasswordRequest {
	email: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ResetPasswordRequest {
	email: String,
	code: String,
	password: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct VerifyEmailRequest {
	email: String,
	code: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ResendVerificationRequest {
	email: String,
}

#[utoipa::path(tag = "auth", security(()), responses((status = 200, description = "If the address matches a local account, a reset email is sent", body = StatusResponse)))]
#[post("/auth/forgot")]
pub async fn forgot_password(
	req: HttpRequest,
	state: web::Data<AppState>,
	body: web::Json<ForgotPasswordRequest>,
) -> Result<HttpResponse, ApiError> {
	enforce_auth_rate_limit(&req, &state)?;
	let state = state.clone();
	let email = body.into_inner().email;
	let accept_language = accept_language(&req);
	actix_web::rt::spawn(async move {
		if let Err(error) = state
			.auth
			.request_password_reset(&email, accept_language.as_deref())
			.await
		{
			tracing::warn!(?error, "password reset request failed");
		}
	});
	Ok(HttpResponse::Ok().json(StatusResponse::new("ok")))
}

#[utoipa::path(tag = "auth", security(()), responses((status = 200, description = "Password reset", body = StatusResponse), (status = 400, description = "Invalid or expired token")))]
#[post("/auth/reset")]
pub async fn reset_password(
	req: HttpRequest,
	state: web::Data<AppState>,
	body: web::Json<ResetPasswordRequest>,
) -> Result<HttpResponse, ApiError> {
	enforce_auth_rate_limit(&req, &state)?;
	state
		.auth
		.reset_password(&body.email, &body.code, &body.password)
		.await?;
	Ok(HttpResponse::Ok().json(StatusResponse::new("ok")))
}

#[utoipa::path(tag = "auth", security(()), responses((status = 200, description = "Email verified", body = StatusResponse), (status = 400, description = "Invalid or expired code")))]
#[post("/auth/verify")]
pub async fn verify_email(
	req: HttpRequest,
	state: web::Data<AppState>,
	body: web::Json<VerifyEmailRequest>,
) -> Result<HttpResponse, ApiError> {
	enforce_auth_rate_limit(&req, &state)?;
	state.auth.verify_email(&body.email, &body.code).await?;
	Ok(HttpResponse::Ok().json(StatusResponse::new("ok")))
}

#[utoipa::path(tag = "auth", security(()), responses((status = 200, description = "If verification is enabled and the address is unverified, a new code is sent", body = StatusResponse)))]
#[post("/auth/verify/resend")]
pub async fn resend_verification(
	req: HttpRequest,
	state: web::Data<AppState>,
	body: web::Json<ResendVerificationRequest>,
) -> Result<HttpResponse, ApiError> {
	enforce_auth_rate_limit(&req, &state)?;
	let state = state.clone();
	let email = body.into_inner().email;
	let accept_language = accept_language(&req);
	actix_web::rt::spawn(async move {
		if let Err(error) = state
			.auth
			.resend_verification(&email, accept_language.as_deref())
			.await
		{
			tracing::warn!(?error, "resend verification failed");
		}
	});
	Ok(HttpResponse::Ok().json(StatusResponse::new("ok")))
}
