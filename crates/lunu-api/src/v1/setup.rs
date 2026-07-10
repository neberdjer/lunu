use actix_web::{HttpResponse, web};
use serde::Deserialize;
use serde_json::json;

use crate::cookie::authenticated_response;
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct SetupRequest {
	username: String,
	password: String,
	email: Option<String>,
}

pub async fn status(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
	let needs_setup = state.auth.needs_setup().await?;
	Ok(HttpResponse::Ok().json(json!({ "needs_setup": needs_setup })))
}

pub async fn create(
	state: web::Data<AppState>,
	body: web::Json<SetupRequest>,
) -> Result<HttpResponse, ApiError> {
	let body = body.into_inner();
	let authenticated = state
		.auth
		.setup_first_admin(&body.username, &body.password, body.email)
		.await?;

	Ok(authenticated_response(
		HttpResponse::Created(),
		&authenticated,
	))
}
