use actix_web::{HttpResponse, post, web};
use lunu_core::services::ManualRequest;
use serde::Deserialize;

use crate::dto::RequestResponse;
use crate::error::ApiError;
use crate::extract::AuthUser;
use crate::state::AppState;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ManualRequestBody {
	title: String,
	#[serde(default)]
	author: Option<String>,
	#[serde(default)]
	notes: Option<String>,
	#[serde(default)]
	quality_profile_id: Option<String>,
}

#[utoipa::path(tag = "requests", responses((status = 201, description = "Manual request created (no ASIN); fulfilled by indexer search on the title", body = RequestResponse)))]
#[post("/requests/manual")]
pub async fn create_manual(
	user: AuthUser,
	state: web::Data<AppState>,
	body: web::Json<ManualRequestBody>,
) -> Result<HttpResponse, ApiError> {
	let body = body.into_inner();
	if let Some(profile_id) = body.quality_profile_id.as_deref() {
		state.quality_profiles.require(profile_id).await?;
	}
	let input = ManualRequest {
		title: body.title,
		author: body.author,
		notes: body.notes,
		quality_profile_id: body.quality_profile_id,
	};
	let request = state.requests.create_manual(&user.0, input).await?;
	Ok(HttpResponse::Created().json(RequestResponse::from(&request)))
}
