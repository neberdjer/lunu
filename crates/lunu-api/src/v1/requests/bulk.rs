use actix_web::{HttpResponse, post, web};
use lunu_core::consts::reasons;
use lunu_core::models::ExternalId;
use lunu_core::services::NewRequest;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::extract::{AdminUser, AuthUser};
use crate::state::AppState;

const MAX_BULK_ITEMS: usize = 100;

fn ensure_within_limit(len: usize) -> Result<(), ApiError> {
	if len == 0 {
		return Err(ApiError::from(lunu_core::Error::Validation(
			reasons::EMPTY_SELECTION.to_string(),
		)));
	}
	if len > MAX_BULK_ITEMS {
		return Err(ApiError::from(lunu_core::Error::Validation(
			reasons::TOO_MANY_ITEMS.to_string(),
		)));
	}
	Ok(())
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct BulkOutcome {
	id: String,
	ok: bool,
	error: Option<String>,
}

impl BulkOutcome {
	fn from_result<T>(id: String, result: Result<T, lunu_core::Error>) -> Self {
		match result {
			Ok(_) => Self {
				id,
				ok: true,
				error: None,
			},
			Err(error) => Self {
				id,
				ok: false,
				error: Some(error.code().to_string()),
			},
		}
	}
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct BulkRequestBody {
	ids: Vec<String>,
	#[serde(default)]
	notes: Option<String>,
	#[serde(default)]
	quality_profile_id: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct BulkIdsBody {
	ids: Vec<String>,
}

#[utoipa::path(tag = "requests", request_body = BulkRequestBody, responses((status = 200, description = "Per-id request outcomes", body = Vec<BulkOutcome>)))]
#[post("/requests/bulk")]
pub async fn bulk_create(
	user: AuthUser,
	state: web::Data<AppState>,
	body: web::Json<BulkRequestBody>,
) -> Result<HttpResponse, ApiError> {
	let body = body.into_inner();
	ensure_within_limit(body.ids.len())?;
	if let Some(profile_id) = body.quality_profile_id.as_deref() {
		state.quality_profiles.require(profile_id).await?;
	}
	let mut outcomes = Vec::with_capacity(body.ids.len());
	for id in body.ids {
		let parsed = match id.parse::<ExternalId>() {
			Ok(parsed) => parsed,
			Err(error) => {
				outcomes.push(BulkOutcome::from_result::<()>(id, Err(error)));
				continue;
			}
		};
		let input = NewRequest {
			id: parsed,
			notes: body.notes.clone(),
			quality_profile_id: body.quality_profile_id.clone(),
		};
		let result = state.requests.create(&user.0, input).await;
		outcomes.push(BulkOutcome::from_result(id, result));
	}
	Ok(HttpResponse::Ok().json(outcomes))
}

#[utoipa::path(tag = "requests", request_body = BulkIdsBody, responses((status = 200, description = "Per-request approval outcomes", body = Vec<BulkOutcome>)))]
#[post("/requests/bulk-approve")]
pub async fn bulk_approve(
	admin: AdminUser,
	state: web::Data<AppState>,
	body: web::Json<BulkIdsBody>,
) -> Result<HttpResponse, ApiError> {
	let ids = body.into_inner().ids;
	ensure_within_limit(ids.len())?;
	let outcomes = bulk_transition(&state, &admin.id, ids, true).await;
	Ok(HttpResponse::Ok().json(outcomes))
}

#[utoipa::path(tag = "requests", request_body = BulkIdsBody, responses((status = 200, description = "Per-request decline outcomes", body = Vec<BulkOutcome>)))]
#[post("/requests/bulk-decline")]
pub async fn bulk_decline(
	admin: AdminUser,
	state: web::Data<AppState>,
	body: web::Json<BulkIdsBody>,
) -> Result<HttpResponse, ApiError> {
	let ids = body.into_inner().ids;
	ensure_within_limit(ids.len())?;
	let outcomes = bulk_transition(&state, &admin.id, ids, false).await;
	Ok(HttpResponse::Ok().json(outcomes))
}

async fn bulk_transition(
	state: &AppState,
	admin_id: &str,
	ids: Vec<String>,
	should_approve: bool,
) -> Vec<BulkOutcome> {
	let mut outcomes = Vec::with_capacity(ids.len());
	for id in ids {
		let result = if should_approve {
			state.requests.approve(admin_id, &id).await
		} else {
			state.requests.decline(admin_id, &id, None).await
		};
		outcomes.push(BulkOutcome::from_result(id, result));
	}
	outcomes
}
