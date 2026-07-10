use actix_web::cookie::time::Duration;
use actix_web::cookie::{Cookie, CookieBuilder, SameSite};
use actix_web::{HttpResponse, HttpResponseBuilder};
use lunu_core::consts::auth::{SESSION_COOKIE, SESSION_TTL_DAYS};
use lunu_core::services::Authenticated;

use crate::dto::UserResponse;

fn base(value: String) -> CookieBuilder<'static> {
	Cookie::build(SESSION_COOKIE, value)
		.http_only(true)
		.same_site(SameSite::Lax)
		.path("/")
}

fn session_cookie(token: String) -> Cookie<'static> {
	base(token)
		.max_age(Duration::days(SESSION_TTL_DAYS))
		.finish()
}

pub(crate) fn clear_session_cookie() -> Cookie<'static> {
	base(String::new()).max_age(Duration::ZERO).finish()
}

pub(crate) fn authenticated_response(
	mut builder: HttpResponseBuilder,
	authenticated: &Authenticated,
) -> HttpResponse {
	builder
		.cookie(session_cookie(authenticated.session_token.clone()))
		.json(UserResponse::from(&authenticated.user))
}
