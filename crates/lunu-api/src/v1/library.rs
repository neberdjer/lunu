use actix_web::{HttpResponse, get, post, web};
use lunu_core::models::{JobType, MediaFilter};
use serde::Deserialize;
use serde_json::json;

use crate::dto::{MediaResponse, MergePreviewResponse};
use crate::error::ApiError;
use crate::extract::{AdminUser, AuthUser};
use crate::pagination::{Page, Pagination};
use crate::state::AppState;

#[derive(Deserialize, utoipa::IntoParams)]
pub struct LibraryQuery {
	#[serde(default)]
	filter: Option<String>,
	#[serde(default)]
	page: Option<i64>,
	#[serde(default)]
	limit: Option<i64>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct MatchBody {
	asin: String,
}

#[utoipa::path(tag = "library", params(LibraryQuery), responses((status = 200, description = "Paginated owned library; filter=unmatched for items with no ASIN, filter=mergeable for items not yet merged", body = Page<MediaResponse>)))]
#[get("/library")]
pub async fn list(
	_user: AuthUser,
	state: web::Data<AppState>,
	query: web::Query<LibraryQuery>,
) -> Result<HttpResponse, ApiError> {
	let filter = match query.filter.as_deref() {
		Some(value) => value.parse()?,
		None => MediaFilter::default(),
	};
	let pagination = Pagination::resolve(query.page, query.limit);
	let (items, total) = state
		.library
		.list(filter, pagination.limit, pagination.offset)
		.await?;
	let items: Vec<MediaResponse> = items.into_iter().map(MediaResponse::from).collect();
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

#[utoipa::path(tag = "library", params(("id" = String, Path, description = "Media item id")), request_body = MatchBody, responses((status = 200, description = "ASIN matched; metadata refreshed from the provider", body = MediaResponse), (status = 404, description = "Unknown media item")))]
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
	Ok(HttpResponse::Ok().json(MediaResponse::from(media)))
}

#[utoipa::path(tag = "library", params(("id" = String, Path, description = "Media item id")), responses((status = 202, description = "Merge enqueued for an already imported item"), (status = 404, description = "Unknown media item"), (status = 400, description = "ffmpeg is not available")))]
#[post("/library/{id}/merge")]
pub async fn merge(
	_admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let job = state.merges.request(&id.into_inner()).await?;
	Ok(HttpResponse::Accepted().json(json!({ "status": "queued", "job_id": job })))
}

#[utoipa::path(tag = "library", params(("id" = String, Path, description = "Media item id")), responses((status = 200, description = "What a merge would produce, without producing it", body = MergePreviewResponse), (status = 404, description = "Unknown media item"), (status = 400, description = "ffmpeg is not available, or the source action is misconfigured")))]
#[get("/library/{id}/merge/preview")]
pub async fn merge_preview(
	_admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let preview = state.merges.preview(&id.into_inner()).await?;
	Ok(HttpResponse::Ok().json(MergePreviewResponse::from(preview)))
}

#[utoipa::path(tag = "library", params(("id" = String, Path, description = "Media item id")), responses((status = 202, description = "Revert enqueued; shelved source files return and the m4b is removed"), (status = 404, description = "Unknown media item"), (status = 400, description = "Nothing to revert, since the sources were deleted or kept in place")))]
#[post("/library/{id}/revert")]
pub async fn revert(
	_admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let job = state.merges.request_revert(&id.into_inner()).await?;
	Ok(HttpResponse::Accepted().json(json!({ "status": "queued", "job_id": job })))
}

#[utoipa::path(tag = "library", responses((status = 202, description = "Merge enqueued for every item not yet merged; safe to repeat, since merged items drop out of the filter"), (status = 400, description = "ffmpeg is not available")))]
#[post("/library/merge-all")]
pub async fn merge_all(
	_admin: AdminUser,
	state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
	let queued = state.merges.request_all().await?;
	Ok(HttpResponse::Accepted().json(json!({
		"status": "queued",
		"queued": queued.queued,
		"truncated": queued.truncated,
	})))
}
