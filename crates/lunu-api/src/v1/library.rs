use actix_web::{HttpResponse, get, post, web};
use lunu_core::models::JobType;
use serde::Deserialize;
use serde_json::json;

use crate::dto::MediaResponse;
use crate::error::ApiError;
use crate::extract::{AdminUser, AuthUser};
use crate::pagination::{Page, Pagination};
use crate::state::AppState;

#[derive(Deserialize, utoipa::IntoParams)]
pub struct LibraryQuery {
	#[serde(default)]
	unmatched: Option<bool>,
	#[serde(default)]
	page: Option<i64>,
	#[serde(default)]
	limit: Option<i64>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct MatchBody {
	asin: String,
}

#[utoipa::path(tag = "library", params(LibraryQuery), responses((status = 200, description = "Paginated owned library; set unmatched=true for items with no ASIN")))]
#[get("/library")]
pub async fn list(
	_user: AuthUser,
	state: web::Data<AppState>,
	query: web::Query<LibraryQuery>,
) -> Result<HttpResponse, ApiError> {
	let pagination = Pagination::resolve(query.page, query.limit);
	let (items, total) = state
		.library
		.list(
			query.unmatched.unwrap_or(false),
			pagination.limit,
			pagination.offset,
		)
		.await?;
	let items: Vec<MediaResponse> = items.iter().map(MediaResponse::from).collect();
	Ok(HttpResponse::Ok().json(Page::new(items, &pagination, total)))
}

#[utoipa::path(tag = "library", responses((status = 202, description = "Audiobookshelf library sync enqueued")))]
#[post("/admin/abs/sync")]
pub async fn sync(_admin: AdminUser, state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
	let enqueued = state.jobs.enqueue_detached(JobType::LibrarySync).await?;
	let status = if enqueued {
		"queued"
	} else {
		"already_running"
	};
	Ok(HttpResponse::Accepted().json(json!({ "status": status })))
}

#[utoipa::path(tag = "library", responses((status = 200, description = "ASIN matched; metadata refreshed from the provider"), (status = 404, description = "Unknown media item")))]
#[post("/library/{id}/match")]
pub async fn match_media(
	_admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
	body: web::Json<MatchBody>,
) -> Result<HttpResponse, ApiError> {
	let media = state
		.library
		.match_asin(&id.into_inner(), &body.asin)
		.await?;
	Ok(HttpResponse::Ok().json(MediaResponse::from(&media)))
}
