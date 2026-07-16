use actix_web::cookie::time::Duration;
use actix_web::cookie::{Cookie, CookieBuilder, SameSite};
use actix_web::{HttpResponse, HttpResponseBuilder};
use lunu_config::BootstrapConfig;
use lunu_core::consts::auth::{SESSION_COOKIE, SESSION_TTL_DAYS};
use lunu_core::services::Authenticated;

use crate::dto::UserResponse;

fn base(value: String, config: &BootstrapConfig) -> CookieBuilder<'static> {
	Cookie::build(SESSION_COOKIE, value)
		.http_only(true)
		.secure(config.secure_cookies)
		.same_site(SameSite::Lax)
		.path(config.base_path().to_string())
}

fn session_cookie(token: String, config: &BootstrapConfig) -> Cookie<'static> {
	base(token, config)
		.max_age(Duration::days(SESSION_TTL_DAYS))
		.finish()
}

pub(crate) fn clear_session_cookie(config: &BootstrapConfig) -> Cookie<'static> {
	base(String::new(), config).max_age(Duration::ZERO).finish()
}

pub(crate) fn authenticated_response(
	mut builder: HttpResponseBuilder,
	authenticated: &Authenticated,
	config: &BootstrapConfig,
) -> HttpResponse {
	builder
		.cookie(session_cookie(authenticated.session_token.clone(), config))
		.json(UserResponse::from(&authenticated.user))
}
