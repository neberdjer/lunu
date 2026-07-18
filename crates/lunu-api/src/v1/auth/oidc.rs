use actix_web::cookie::Cookie;
use actix_web::cookie::time::Duration;
use actix_web::{HttpRequest, HttpResponse, get, web};
use lunu_core::Error;
use lunu_core::consts::auth::OIDC_STATE_TTL_MINS;
use lunu_core::consts::reasons;
use serde::Deserialize;

use crate::cookie::{authenticated_response_redirect, cookie_base};
use crate::error::ApiError;
use crate::state::AppState;

const BINDING_COOKIE: &str = "lunu_oidc";

#[derive(Deserialize, utoipa::IntoParams)]
pub struct CallbackQuery {
	state: String,
	code: String,
}

fn binding_cookie(value: String, state: &AppState, max_age: Duration) -> Cookie<'static> {
	cookie_base(BINDING_COOKIE, value, &state.config)
		.max_age(max_age)
		.finish()
}

#[utoipa::path(tag = "auth", security(()), responses((status = 302, description = "Redirect to the identity provider")))]
#[get("/auth/oidc/start")]
pub async fn start(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
	let started = state.auth.oidc_start().await?;
	Ok(HttpResponse::Found()
		.cookie(binding_cookie(
			started.binding,
			&state,
			Duration::minutes(OIDC_STATE_TTL_MINS),
		))
		.insert_header(("Location", started.url))
		.finish())
}

#[utoipa::path(tag = "auth", security(()), params(CallbackQuery), responses((status = 302, description = "Session cookie set; redirect to the app root")))]
#[get("/auth/oidc/callback")]
pub async fn callback(
	req: HttpRequest,
	state: web::Data<AppState>,
	query: web::Query<CallbackQuery>,
) -> Result<HttpResponse, ApiError> {
	let binding = req
		.cookie(BINDING_COOKIE)
		.map(|cookie| cookie.value().to_string())
		.ok_or_else(|| Error::Validation(reasons::OIDC_STATE_INVALID.to_string()))?;
	let authenticated = state
		.auth
		.oidc_callback(&query.state, &query.code, &binding)
		.await?;
	let mut response = authenticated_response_redirect(&authenticated, &state.config);
	let _ = response.add_cookie(&binding_cookie(String::new(), &state, Duration::ZERO));
	Ok(response)
}
