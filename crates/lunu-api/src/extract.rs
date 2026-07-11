use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;

use actix_web::http::header::AUTHORIZATION;
use actix_web::{FromRequest, HttpRequest, dev::Payload, web};
use lunu_core::consts::auth::{API_KEY_HEADER, BEARER_PREFIX, SCOPE_ADMIN, SESSION_COOKIE};
use lunu_core::models::User;
use lunu_core::{Error, Result};

use crate::error::ApiError;
use crate::state::AppState;

pub struct AuthUser(pub User);

impl Deref for AuthUser {
	type Target = User;

	fn deref(&self) -> &User {
		&self.0
	}
}

pub struct AdminUser(pub User);

impl Deref for AdminUser {
	type Target = User;

	fn deref(&self) -> &User {
		&self.0
	}
}

impl FromRequest for AuthUser {
	type Error = ApiError;
	type Future = Pin<Box<dyn Future<Output = std::result::Result<Self, Self::Error>>>>;

	fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
		let req = req.clone();
		Box::pin(async move {
			let state = app_state(&req);
			let (user, _scopes) = resolve_identity(&req, &state).await?;
			Ok(AuthUser(user))
		})
	}
}

impl FromRequest for AdminUser {
	type Error = ApiError;
	type Future = Pin<Box<dyn Future<Output = std::result::Result<Self, Self::Error>>>>;

	fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
		let req = req.clone();
		Box::pin(async move {
			let state = app_state(&req);
			let (user, scopes) = resolve_identity(&req, &state).await?;
			if !user.role.is_admin() {
				return Err(Error::Forbidden.into());
			}
			if let Some(scopes) = scopes
				&& !scopes.iter().any(|scope| scope == SCOPE_ADMIN)
			{
				return Err(Error::Forbidden.into());
			}
			Ok(AdminUser(user))
		})
	}
}

fn app_state(req: &HttpRequest) -> web::Data<AppState> {
	req.app_data::<web::Data<AppState>>()
		.expect("AppState is configured on the application")
		.clone()
}

async fn resolve_identity(
	req: &HttpRequest,
	state: &AppState,
) -> std::result::Result<(User, Option<Vec<String>>), ApiError> {
	if let Some(user) = user_from_session(req, state).await? {
		return Ok((user, None));
	}
	if let Some((user, scopes)) = user_from_api_key(req, state).await? {
		return Ok((user, Some(scopes)));
	}
	Err(Error::Unauthorized.into())
}

async fn user_from_session(req: &HttpRequest, state: &AppState) -> Result<Option<User>> {
	let Some(cookie) = req.cookie(SESSION_COOKIE) else {
		return Ok(None);
	};
	state.auth.validate_session(cookie.value()).await
}

async fn user_from_api_key(
	req: &HttpRequest,
	state: &AppState,
) -> Result<Option<(User, Vec<String>)>> {
	let Some(secret) = api_key_from_headers(req) else {
		return Ok(None);
	};

	let Some(key) = state.api_keys.verify(&secret).await? else {
		return Ok(None);
	};

	let Some(user) = state.users.get(&key.user_id).await? else {
		return Ok(None);
	};

	if !user.enabled {
		return Ok(None);
	}

	Ok(Some((user, key.scopes)))
}

fn api_key_from_headers(req: &HttpRequest) -> Option<String> {
	if let Some(value) = req
		.headers()
		.get(API_KEY_HEADER)
		.and_then(|value| value.to_str().ok())
	{
		return Some(value.to_string());
	}

	let header = req.headers().get(AUTHORIZATION)?.to_str().ok()?;
	header.strip_prefix(BEARER_PREFIX).map(str::to_string)
}
