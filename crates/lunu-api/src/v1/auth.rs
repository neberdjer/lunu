use actix_web::{HttpRequest, HttpResponse, delete, get, patch, post, web};
use lunu_core::Error;
use lunu_core::consts::auth::SESSION_COOKIE;
use serde::Deserialize;
use serde_json::json;

use lunu_core::services::Registration;

use crate::cookie::{authenticated_response, clear_session_cookie};
use crate::dto::{SessionResponse, UserResponse};
use crate::error::ApiError;
use crate::extract::{AuthUser, accept_language, user_agent};
use crate::state::AppState;

fn enforce_auth_rate_limit(req: &HttpRequest, state: &AppState) -> Result<(), ApiError> {
	let ip = crate::client_ip::client_ip(req, &state.config);
	if state.auth_rate_limiter.check(&ip) {
		Ok(())
	} else {
		Err(Error::RateLimited.into())
	}
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct LoginRequest {
	username: String,
	password: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct RegisterRequest {
	code: String,
	username: String,
	password: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ChangePasswordRequest {
	current_password: String,
	new_password: String,
}

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

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateProfileRequest {
	email: Option<String>,
	#[serde(default)]
	display_name: Option<String>,
	#[serde(default)]
	locale: Option<String>,
}

#[utoipa::path(tag = "auth", security(()), responses((status = 200, description = "Authenticated, session cookie set", body = UserResponse), (status = 401, description = "Invalid credentials")))]
#[post("/auth/login")]
pub async fn login(
	req: HttpRequest,
	state: web::Data<AppState>,
	body: web::Json<LoginRequest>,
) -> Result<HttpResponse, ApiError> {
	enforce_auth_rate_limit(&req, &state)?;
	let authenticated = state.auth.login(&body.username, &body.password).await?;
	state
		.auth
		.record_login_device(
			&authenticated.user,
			&authenticated.session_id,
			user_agent(&req).as_deref(),
			accept_language(&req).as_deref(),
		)
		.await
		.ok();
	Ok(authenticated_response(
		HttpResponse::Ok(),
		&authenticated,
		state.config.secure_cookies,
	))
}

#[utoipa::path(tag = "auth", security(()), responses((status = 201, description = "Registered via invite; session set, or verification pending", body = UserResponse)))]
#[post("/auth/register")]
pub async fn register(
	req: HttpRequest,
	state: web::Data<AppState>,
	body: web::Json<RegisterRequest>,
) -> Result<HttpResponse, ApiError> {
	enforce_auth_rate_limit(&req, &state)?;
	let outcome = state
		.auth
		.register_with_invite(
			&body.code,
			&body.username,
			&body.password,
			accept_language(&req).as_deref(),
		)
		.await?;
	match outcome {
		Registration::Active(authenticated) => {
			state
				.auth
				.record_user_agent(&authenticated.session_id, user_agent(&req).as_deref())
				.await
				.ok();
			Ok(authenticated_response(
				HttpResponse::Created(),
				&authenticated,
				state.config.secure_cookies,
			))
		}
		Registration::PendingVerification => {
			Ok(HttpResponse::Created().json(json!({ "status": "pending_verification" })))
		}
	}
}

#[utoipa::path(tag = "auth", security(()), responses((status = 200, description = "Session cleared")))]
#[post("/auth/logout")]
pub async fn logout(
	req: HttpRequest,
	state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
	if let Some(cookie) = req.cookie(SESSION_COOKIE) {
		state.auth.logout(cookie.value()).await?;
	}
	Ok(HttpResponse::Ok()
		.cookie(clear_session_cookie(state.config.secure_cookies))
		.json(json!({ "status": "ok" })))
}

#[utoipa::path(tag = "auth", responses((status = 200, body = UserResponse)))]
#[get("/auth/me")]
pub async fn me(user: AuthUser) -> HttpResponse {
	HttpResponse::Ok().json(UserResponse::from(&user.0))
}

#[utoipa::path(tag = "auth", responses((status = 200, description = "Email updated", body = UserResponse)))]
#[patch("/auth/me")]
pub async fn update_me(
	user: AuthUser,
	state: web::Data<AppState>,
	body: web::Json<UpdateProfileRequest>,
) -> Result<HttpResponse, ApiError> {
	let body = body.into_inner();
	let updated = state
		.users
		.update_profile(&user.0.id, body.email, body.display_name, body.locale)
		.await?;
	Ok(HttpResponse::Ok().json(UserResponse::from(&updated)))
}

#[utoipa::path(tag = "auth", responses((status = 200, description = "Password changed and sessions rotated", body = UserResponse)))]
#[post("/auth/password")]
pub async fn change_password(
	req: HttpRequest,
	user: AuthUser,
	state: web::Data<AppState>,
	body: web::Json<ChangePasswordRequest>,
) -> Result<HttpResponse, ApiError> {
	enforce_auth_rate_limit(&req, &state)?;
	let authenticated = state
		.auth
		.change_password(&user.0.id, &body.current_password, &body.new_password)
		.await?;
	state
		.auth
		.record_user_agent(&authenticated.session_id, user_agent(&req).as_deref())
		.await
		.ok();
	Ok(authenticated_response(
		HttpResponse::Ok(),
		&authenticated,
		state.config.secure_cookies,
	))
}

#[utoipa::path(tag = "auth", security(()), responses((status = 200, description = "If the address matches a local account, a reset email is sent")))]
#[post("/auth/forgot")]
pub async fn forgot_password(
	req: HttpRequest,
	state: web::Data<AppState>,
	body: web::Json<ForgotPasswordRequest>,
) -> Result<HttpResponse, ApiError> {
	enforce_auth_rate_limit(&req, &state)?;
	let accept_language = req
		.headers()
		.get(actix_web::http::header::ACCEPT_LANGUAGE)
		.and_then(|value| value.to_str().ok());
	state
		.auth
		.request_password_reset(&body.email, accept_language)
		.await?;
	Ok(HttpResponse::Ok().json(json!({ "status": "ok" })))
}

#[utoipa::path(tag = "auth", security(()), responses((status = 200, description = "Password reset"), (status = 400, description = "Invalid or expired token")))]
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
	Ok(HttpResponse::Ok().json(json!({ "status": "ok" })))
}

#[utoipa::path(tag = "auth", security(()), responses((status = 200, description = "Email verified"), (status = 400, description = "Invalid or expired code")))]
#[post("/auth/verify")]
pub async fn verify_email(
	req: HttpRequest,
	state: web::Data<AppState>,
	body: web::Json<VerifyEmailRequest>,
) -> Result<HttpResponse, ApiError> {
	enforce_auth_rate_limit(&req, &state)?;
	state.auth.verify_email(&body.email, &body.code).await?;
	Ok(HttpResponse::Ok().json(json!({ "status": "ok" })))
}

#[utoipa::path(tag = "auth", security(()), responses((status = 200, description = "If verification is enabled and the address is unverified, a new code is sent")))]
#[post("/auth/verify/resend")]
pub async fn resend_verification(
	req: HttpRequest,
	state: web::Data<AppState>,
	body: web::Json<ResendVerificationRequest>,
) -> Result<HttpResponse, ApiError> {
	enforce_auth_rate_limit(&req, &state)?;
	state
		.auth
		.resend_verification(&body.email, accept_language(&req).as_deref())
		.await?;
	Ok(HttpResponse::Ok().json(json!({ "status": "ok" })))
}

#[utoipa::path(tag = "auth", responses((status = 200, description = "Active sessions for the caller", body = Vec<SessionResponse>)))]
#[get("/auth/sessions")]
pub async fn sessions(
	req: HttpRequest,
	user: AuthUser,
	state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
	let current = match req.cookie(SESSION_COOKIE) {
		Some(cookie) => state.auth.current_session_id(cookie.value()).await?,
		None => None,
	};
	let sessions = state.auth.list_sessions(&user.0.id).await?;
	let items: Vec<SessionResponse> = sessions
		.iter()
		.map(|session| SessionResponse::new(session, current.as_deref() == Some(&session.id)))
		.collect();
	Ok(HttpResponse::Ok().json(items))
}

#[utoipa::path(tag = "auth", responses((status = 204, description = "Session revoked"), (status = 404, description = "Not found")))]
#[delete("/auth/sessions/{id}")]
pub async fn revoke_session(
	user: AuthUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	state
		.auth
		.revoke_session(&user.0.id, &id.into_inner())
		.await?;
	Ok(HttpResponse::NoContent().finish())
}
