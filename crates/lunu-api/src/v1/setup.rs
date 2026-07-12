use actix_web::{HttpRequest, HttpResponse, get, post, web};
use serde::Deserialize;
use serde_json::json;

use crate::cookie::authenticated_response;
use crate::dto::UserResponse;
use crate::error::ApiError;
use crate::extract::user_agent;
use crate::state::AppState;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct SetupRequest {
	username: String,
	password: String,
	email: Option<String>,
}

#[utoipa::path(
	tag = "setup",
	security(()),
	responses((status = 200, description = "Whether the first-run admin setup is still required"))
)]
#[get("/setup")]
pub async fn status(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
	let needs_setup = state.auth.needs_setup().await?;
	Ok(HttpResponse::Ok().json(json!({ "needs_setup": needs_setup })))
}

#[utoipa::path(
	tag = "setup",
	security(()),
	responses(
		(status = 201, description = "First admin created and session issued", body = UserResponse),
		(status = 409, description = "Setup already completed")
	)
)]
#[post("/setup")]
pub async fn create(
	req: HttpRequest,
	state: web::Data<AppState>,
	body: web::Json<SetupRequest>,
) -> Result<HttpResponse, ApiError> {
	let body = body.into_inner();
	let authenticated = state
		.auth
		.setup_first_admin(&body.username, &body.password, body.email)
		.await?;
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
