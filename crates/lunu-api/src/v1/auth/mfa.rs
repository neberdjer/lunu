use actix_web::{HttpRequest, HttpResponse, delete, get, post, web};
use lunu_core::consts::auth::SESSION_COOKIE;
use lunu_core::models::MfaMethod;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::extract::{AdminUser, AuthUser, accept_language};
use crate::state::AppState;

async fn current_session(req: &HttpRequest, state: &AppState) -> Result<Option<String>, ApiError> {
	match req.cookie(SESSION_COOKIE) {
		Some(cookie) => Ok(state.auth.current_session_id(cookie.value()).await?),
		None => Ok(None),
	}
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct EnrollRequest {
	method: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct EnrollResponse {
	method: String,
	secret: Option<String>,
	otpauth_uri: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ConfirmRequest {
	code: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct MfaStatusResponse {
	enabled: bool,
	method: Option<String>,
	recovery_codes_remaining: Option<i64>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct RecoveryCodesResponse {
	recovery_codes: Vec<String>,
}

#[utoipa::path(tag = "auth", responses((status = 200, description = "Two-factor status", body = MfaStatusResponse)))]
#[get("/auth/mfa")]
pub async fn status(user: AuthUser, state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
	let status = state.auth.mfa_status(&user.0).await?;
	Ok(HttpResponse::Ok().json(MfaStatusResponse {
		enabled: status.enabled,
		method: status.method.map(|method| method.as_str().to_string()),
		recovery_codes_remaining: status.recovery_codes_remaining,
	}))
}

#[utoipa::path(tag = "auth", request_body = EnrollRequest, responses((status = 200, description = "Enrollment started", body = EnrollResponse)))]
#[post("/auth/mfa/enroll")]
pub async fn enroll(
	req: HttpRequest,
	user: AuthUser,
	state: web::Data<AppState>,
	body: web::Json<EnrollRequest>,
) -> Result<HttpResponse, ApiError> {
	let method: MfaMethod = body.method.parse()?;
	let enrollment = state.auth.mfa_begin_enrollment(&user.0, method).await?;
	if method == MfaMethod::Email {
		state
			.auth
			.mfa_send_enrollment_code(&user.0, accept_language(&req).as_deref())
			.await?;
	}
	Ok(HttpResponse::Ok().json(EnrollResponse {
		method: enrollment.method.as_str().to_string(),
		secret: enrollment.secret,
		otpauth_uri: enrollment.otpauth_uri,
	}))
}

#[utoipa::path(tag = "auth", request_body = ConfirmRequest, responses((status = 200, description = "Two-factor enabled, recovery codes issued", body = RecoveryCodesResponse)))]
#[post("/auth/mfa/confirm")]
pub async fn confirm(
	req: HttpRequest,
	user: AuthUser,
	state: web::Data<AppState>,
	body: web::Json<ConfirmRequest>,
) -> Result<HttpResponse, ApiError> {
	let current = current_session(&req, &state).await?;
	let recovery_codes = state
		.auth
		.mfa_confirm_enrollment(&user.0, &body.code, current.as_deref())
		.await?;
	Ok(HttpResponse::Ok().json(RecoveryCodesResponse { recovery_codes }))
}

#[utoipa::path(tag = "auth", responses((status = 200, description = "Recovery codes regenerated", body = RecoveryCodesResponse)))]
#[post("/auth/mfa/recovery-codes")]
pub async fn regenerate_recovery_codes(
	user: AuthUser,
	state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
	let recovery_codes = state.auth.mfa_regenerate_recovery_codes(&user.0).await?;
	Ok(HttpResponse::Ok().json(RecoveryCodesResponse { recovery_codes }))
}

#[utoipa::path(tag = "auth", responses((status = 204, description = "Two-factor disabled")))]
#[delete("/auth/mfa")]
pub async fn disable(
	req: HttpRequest,
	user: AuthUser,
	state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
	let current = current_session(&req, &state).await?;
	state.auth.mfa_disable(&user.0, current.as_deref()).await?;
	Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(tag = "auth", params(("user_id" = String, Path, description = "User whose two-factor to reset")), responses((status = 204, description = "Two-factor reset for the user")))]
#[delete("/auth/mfa/users/{user_id}")]
pub async fn admin_reset(
	_admin: AdminUser,
	state: web::Data<AppState>,
	user_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	state.auth.mfa_admin_reset(&user_id).await?;
	Ok(HttpResponse::NoContent().finish())
}
