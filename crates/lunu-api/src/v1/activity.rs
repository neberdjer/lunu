use actix_web::{HttpResponse, web};
use lunu_core::consts::api::ACTIVITY_FEED_LIMIT;

use crate::dto::ActivityResponse;
use crate::error::ApiError;
use crate::extract::AdminUser;
use crate::state::AppState;

pub async fn list(_admin: AdminUser, state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
	let activity = state.activity.recent(ACTIVITY_FEED_LIMIT).await?;
	let response: Vec<ActivityResponse> = activity.iter().map(ActivityResponse::from).collect();
	Ok(HttpResponse::Ok().json(response))
}
