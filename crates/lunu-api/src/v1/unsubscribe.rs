use actix_web::{HttpResponse, post, web};

use crate::dto::StatusResponse;
use crate::error::ApiError;
use crate::state::AppState;

#[utoipa::path(
	tag = "auth",
	security(()),
	params(("token" = String, Path, description = "One-click unsubscribe token")),
	responses(
		(status = 200, description = "Notification emails turned off for the token's account", body = StatusResponse),
		(status = 400, description = "Invalid or expired token")
	)
)]
#[post("/unsubscribe/{token}")]
pub async fn unsubscribe(
	state: web::Data<AppState>,
	token: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let user_id = state
		.unsubscribe
		.decrypt_token(&token.into_inner())
		.map_err(|_| {
			lunu_core::Error::Validation(
				lunu_core::consts::reasons::UNSUBSCRIBE_TOKEN_INVALID.to_string(),
			)
		})?;
	state.users.set_notify_email(&user_id, false).await?;
	Ok(HttpResponse::Ok().json(StatusResponse::new("unsubscribed")))
}
