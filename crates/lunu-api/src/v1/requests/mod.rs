use std::str::FromStr;

use actix_web::{HttpResponse, delete, get, post, web};
use lunu_core::models::ExternalId;
use lunu_core::models::RequestStatus;
use lunu_core::services::{NewRequest, ReleaseSelection};
use serde::Deserialize;

use crate::dto::{
	ActivityResponse, BlocklistResponse, DownloadResponse, RequestResponse, ScoredReleaseResponse,
};
use crate::error::ApiError;
use crate::extract::{AdminUser, AuthUser};
use crate::pagination::{Page, Pagination};
use crate::state::AppState;

mod bulk;
mod manual;

pub use bulk::{bulk_approve, bulk_create, bulk_decline};
pub use manual::create_manual;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateRequestBody {
	id: String,
	#[serde(default)]
	notes: Option<String>,
	#[serde(default)]
	quality_profile_id: Option<String>,
}

#[derive(Deserialize, Default, utoipa::ToSchema)]
pub struct DeclineBody {
	#[serde(default)]
	reason: Option<String>,
}

#[derive(Deserialize, Default, utoipa::ToSchema)]
pub struct GrabBody {
	download_url: Option<String>,
	#[serde(default)]
	title: String,
	#[serde(default)]
	indexer: String,
	#[serde(default)]
	info_hash: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct BlocklistBody {
	download_url: String,
}

#[utoipa::path(tag = "requests", params(("id" = String, Path, description = "Request id")), responses((status = 204, description = "Request deleted")))]
#[delete("/requests/{id}")]
pub async fn delete(
	user: AuthUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	state.requests.delete(&user, &id.into_inner()).await?;
	Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(tag = "requests", params(("id" = String, Path, description = "Request id")), responses((status = 200, description = "Failed request reopened", body = RequestResponse)))]
#[post("/requests/{id}/retry")]
pub async fn retry(
	user: AuthUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let request = state.requests.retry(&user, &id.into_inner()).await?;
	Ok(HttpResponse::Ok().json(RequestResponse::from(&request)))
}

#[utoipa::path(tag = "requests", params(("id" = String, Path, description = "Request id")), request_body = BlocklistBody, responses((status = 204, description = "Release blocklisted")))]
#[post("/requests/{id}/blocklist")]
pub async fn blocklist(
	_admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
	body: web::Json<BlocklistBody>,
) -> Result<HttpResponse, ApiError> {
	state
		.releases
		.blocklist_release(&id.into_inner(), &body.download_url)
		.await?;
	Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(tag = "requests", params(("id" = String, Path, description = "Request id")), responses((status = 200, description = "Blocklisted releases for the request", body = Vec<BlocklistResponse>)))]
#[get("/requests/{id}/blocklist")]
pub async fn blocklist_list(
	_admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let entries = state.releases.list_blocklist(&id.into_inner()).await?;
	let items: Vec<BlocklistResponse> = entries.iter().map(BlocklistResponse::from).collect();
	Ok(HttpResponse::Ok().json(items))
}

#[utoipa::path(tag = "requests", params(("id" = String, Path, description = "Request id"), ("entry_id" = String, Path, description = "Blocklist entry id")), responses((status = 204, description = "Blocklist entry removed"), (status = 404, description = "Not found")))]
#[delete("/requests/{id}/blocklist/{entry_id}")]
pub async fn blocklist_remove(
	_admin: AdminUser,
	state: web::Data<AppState>,
	path: web::Path<(String, String)>,
) -> Result<HttpResponse, ApiError> {
	let (id, entry_id) = path.into_inner();
	state.releases.remove_blocklist(&id, &entry_id).await?;
	Ok(HttpResponse::NoContent().finish())
}

#[derive(Deserialize, utoipa::IntoParams)]
pub struct RequestListParams {
	page: Option<i64>,
	limit: Option<i64>,
	status: Option<String>,
}

#[utoipa::path(tag = "requests", params(RequestListParams), responses((status = 200, body = Page<RequestResponse>)))]
#[get("/requests")]
pub async fn list(
	user: AuthUser,
	query: web::Query<RequestListParams>,
	state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
	let pagination = Pagination::resolve(query.page, query.limit);
	let status = query
		.status
		.as_deref()
		.map(RequestStatus::from_str)
		.transpose()?;

	let (requests, total) = tokio::try_join!(
		state
			.requests
			.list_page(&user, status, pagination.limit, pagination.offset),
		state.requests.count(&user, status),
	)?;

	let items: Vec<RequestResponse> = requests.iter().map(RequestResponse::from).collect();
	Ok(HttpResponse::Ok().json(Page::new(items, &pagination, total)))
}

#[utoipa::path(tag = "requests", request_body = CreateRequestBody, responses((status = 201, description = "Request created", body = RequestResponse)))]
#[post("/requests")]
pub async fn create(
	user: AuthUser,
	state: web::Data<AppState>,
	body: web::Json<CreateRequestBody>,
) -> Result<HttpResponse, ApiError> {
	let body = body.into_inner();
	if let Some(profile_id) = body.quality_profile_id.as_deref() {
		state.quality_profiles.require(profile_id).await?;
	}
	let input = NewRequest {
		id: body.id.parse::<ExternalId>()?,
		notes: body.notes,
		quality_profile_id: body.quality_profile_id,
	};
	let request = state.requests.create(&user.0, input).await?;
	Ok(HttpResponse::Created().json(RequestResponse::from(&request)))
}

#[utoipa::path(tag = "requests", params(("id" = String, Path, description = "Request id")), responses((status = 200, body = RequestResponse), (status = 404, description = "Not found")))]
#[get("/requests/{id}")]
pub async fn get(
	user: AuthUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let id = id.into_inner();
	let request = state.requests.get_for(&user, &id).await?;
	Ok(HttpResponse::Ok().json(RequestResponse::from(&request)))
}

#[utoipa::path(tag = "requests", params(("id" = String, Path, description = "Request id")), responses((status = 204, description = "Download cancelled"), (status = 404, description = "No download")))]
#[delete("/requests/{id}/download")]
pub async fn cancel_download(
	_admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	state.grabs.cancel(&id.into_inner()).await?;
	Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(tag = "requests", params(("id" = String, Path, description = "Request id")), responses((status = 200, description = "Download progress", body = DownloadResponse), (status = 404, description = "No download")))]
#[get("/requests/{id}/download")]
pub async fn request_download(
	user: AuthUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let id = id.into_inner();
	state.requests.get_for(&user, &id).await?;
	let download = state
		.grabs
		.for_request(&id)
		.await?
		.ok_or_else(|| lunu_core::Error::NotFound(format!("download for request {id}")))?;
	Ok(HttpResponse::Ok().json(DownloadResponse::from(&download)))
}

#[utoipa::path(tag = "requests", params(("id" = String, Path, description = "Request id")), responses((status = 200, description = "Status timeline", body = Vec<ActivityResponse>)))]
#[get("/requests/{id}/activity")]
pub async fn activity(
	user: AuthUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let id = id.into_inner();
	state.requests.get_for(&user, &id).await?;

	let activity = state.activity.for_request(&id).await?;
	let items: Vec<ActivityResponse> = activity.iter().map(ActivityResponse::from).collect();
	Ok(HttpResponse::Ok().json(items))
}

#[utoipa::path(tag = "requests", params(("id" = String, Path, description = "Request id")), responses((status = 200, description = "Approved", body = RequestResponse)))]
#[post("/requests/{id}/approve")]
pub async fn approve(
	admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let request = state.requests.approve(&admin.id, &id).await?;
	Ok(HttpResponse::Ok().json(RequestResponse::from(&request)))
}

#[utoipa::path(tag = "requests", params(("id" = String, Path, description = "Request id")), request_body = DeclineBody, responses((status = 200, description = "Declined", body = RequestResponse)))]
#[post("/requests/{id}/decline")]
pub async fn decline(
	admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
	body: Option<web::Json<DeclineBody>>,
) -> Result<HttpResponse, ApiError> {
	let reason = body.and_then(|body| body.into_inner().reason);
	let request = state.requests.decline(&admin.id, &id, reason).await?;
	Ok(HttpResponse::Ok().json(RequestResponse::from(&request)))
}

#[utoipa::path(tag = "requests", params(("id" = String, Path, description = "Request id")), responses((status = 200, description = "Ranked releases for the request", body = Vec<ScoredReleaseResponse>)))]
#[get("/requests/{id}/releases")]
pub async fn releases(
	_admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let releases = state.releases.for_request(&id).await?;
	let items: Vec<ScoredReleaseResponse> =
		releases.iter().map(ScoredReleaseResponse::from).collect();
	Ok(HttpResponse::Ok().json(items))
}

#[utoipa::path(tag = "requests", params(("id" = String, Path, description = "Request id")), request_body = GrabBody, responses((status = 201, description = "Grab enqueued", body = DownloadResponse)))]
#[post("/requests/{id}/grab")]
pub async fn grab(
	_admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
	body: web::Json<GrabBody>,
) -> Result<HttpResponse, ApiError> {
	let body = body.into_inner();
	let selection = body.download_url.map(|download_url| ReleaseSelection {
		download_url,
		title: body.title,
		indexer: body.indexer,
		info_hash: body.info_hash,
	});

	let download = state.grabs.grab(&id, selection).await?;
	Ok(HttpResponse::Created().json(DownloadResponse::from(&download)))
}
