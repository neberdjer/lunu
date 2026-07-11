use actix_web::{HttpRequest, HttpResponse, web};
use lunu_core::Error;
use lunu_core::consts::auth::SESSION_COOKIE;
use serde::Deserialize;
use serde_json::json;

use crate::cookie::{authenticated_response, clear_session_cookie};
use crate::dto::UserResponse;
use crate::error::ApiError;
use crate::extract::AuthUser;
use crate::state::AppState;

fn enforce_auth_rate_limit(req: &HttpRequest, state: &AppState) -> Result<(), ApiError> {
	let ip = req
		.connection_info()
		.realip_remote_addr()
		.map(str::to_string)
		.unwrap_or_default();
	if state.auth_rate_limiter.check(&ip) {
		Ok(())
	} else {
		Err(Error::RateLimited.into())
	}
}

#[derive(Deserialize)]
pub struct LoginRequest {
	username: String,
	password: String,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
	code: String,
	username: String,
	password: String,
}

#[derive(Deserialize)]
pub struct ChangePasswordRequest {
	current_password: String,
	new_password: String,
}

#[derive(Deserialize)]
pub struct UpdateProfileRequest {
	email: Option<String>,
}

pub async fn login(
	req: HttpRequest,
	state: web::Data<AppState>,
	body: web::Json<LoginRequest>,
) -> Result<HttpResponse, ApiError> {
	enforce_auth_rate_limit(&req, &state)?;
	let authenticated = state.auth.login(&body.username, &body.password).await?;
	Ok(authenticated_response(HttpResponse::Ok(), &authenticated))
}

pub async fn register(
	req: HttpRequest,
	state: web::Data<AppState>,
	body: web::Json<RegisterRequest>,
) -> Result<HttpResponse, ApiError> {
	enforce_auth_rate_limit(&req, &state)?;
	let authenticated = state
		.auth
		.register_with_invite(&body.code, &body.username, &body.password)
		.await?;
	Ok(authenticated_response(
		HttpResponse::Created(),
		&authenticated,
	))
}

pub async fn logout(
	req: HttpRequest,
	state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
	if let Some(cookie) = req.cookie(SESSION_COOKIE) {
		state.auth.logout(cookie.value()).await?;
	}
	Ok(HttpResponse::Ok()
		.cookie(clear_session_cookie())
		.json(json!({ "status": "ok" })))
}

pub async fn me(user: AuthUser) -> HttpResponse {
	HttpResponse::Ok().json(UserResponse::from(&user.0))
}

pub async fn update_me(
	user: AuthUser,
	state: web::Data<AppState>,
	body: web::Json<UpdateProfileRequest>,
) -> Result<HttpResponse, ApiError> {
	let updated = state
		.users
		.update_email(&user.0.id, body.into_inner().email)
		.await?;
	Ok(HttpResponse::Ok().json(UserResponse::from(&updated)))
}

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
	Ok(authenticated_response(HttpResponse::Ok(), &authenticated))
}
