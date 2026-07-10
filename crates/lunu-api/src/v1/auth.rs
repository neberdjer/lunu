use actix_web::{HttpRequest, HttpResponse, web};
use lunu_core::consts::auth::SESSION_COOKIE;
use serde::Deserialize;
use serde_json::json;

use crate::cookie::{authenticated_response, clear_session_cookie};
use crate::dto::UserResponse;
use crate::error::ApiError;
use crate::extract::AuthUser;
use crate::state::AppState;

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

pub async fn login(
	state: web::Data<AppState>,
	body: web::Json<LoginRequest>,
) -> Result<HttpResponse, ApiError> {
	let authenticated = state.auth.login(&body.username, &body.password).await?;
	Ok(authenticated_response(HttpResponse::Ok(), &authenticated))
}

pub async fn register(
	state: web::Data<AppState>,
	body: web::Json<RegisterRequest>,
) -> Result<HttpResponse, ApiError> {
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
