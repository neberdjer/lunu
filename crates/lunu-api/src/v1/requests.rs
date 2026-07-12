use std::str::FromStr;

use actix_web::{HttpResponse, delete, get, post, web};
use lunu_core::models::RequestStatus;
use lunu_core::services::ReleaseSelection;
use serde::Deserialize;

use crate::dto::{ActivityResponse, DownloadResponse, RequestResponse};
use crate::error::ApiError;
use crate::extract::{AdminUser, AuthUser};
use crate::pagination::{Page, Pagination};
use crate::state::AppState;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateRequestBody {
	asin: String,
}

#[derive(Deserialize, Default, utoipa::ToSchema)]
pub struct GrabBody {
	download_url: Option<String>,
	#[serde(default)]
	title: String,
	#[serde(default)]
	indexer: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct BlocklistBody {
	download_url: String,
}

#[utoipa::path(tag = "requests", responses((status = 204, description = "Request deleted")))]
#[delete("/requests/{id}")]
pub async fn delete(
	user: AuthUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	state.requests.delete(&user, &id.into_inner()).await?;
	Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(tag = "requests", responses((status = 200, description = "Failed request reopened", body = RequestResponse)))]
#[post("/requests/{id}/retry")]
pub async fn retry(
	user: AuthUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let request = state.requests.retry(&user, &id.into_inner()).await?;
	Ok(HttpResponse::Ok().json(RequestResponse::from(&request)))
}

#[utoipa::path(tag = "requests", responses((status = 204, description = "Release blocklisted")))]
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

	let requests = state
		.requests
		.list_page(&user, status, pagination.limit, pagination.offset)
		.await?;
	let total = state.requests.count(&user, status).await?;

	let items: Vec<RequestResponse> = requests.iter().map(RequestResponse::from).collect();
	Ok(HttpResponse::Ok().json(Page::new(items, &pagination, total)))
}

#[utoipa::path(tag = "requests", responses((status = 201, description = "Request created", body = RequestResponse)))]
#[post("/requests")]
pub async fn create(
	user: AuthUser,
	state: web::Data<AppState>,
	body: web::Json<CreateRequestBody>,
) -> Result<HttpResponse, ApiError> {
	let request = state.requests.create(&user.0, &body.asin).await?;
	Ok(HttpResponse::Created().json(RequestResponse::from(&request)))
}

#[utoipa::path(tag = "requests", responses((status = 200, body = RequestResponse), (status = 404, description = "Not found")))]
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

#[utoipa::path(tag = "requests", responses((status = 200, description = "Status timeline", body = Vec<ActivityResponse>)))]
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

#[utoipa::path(tag = "requests", responses((status = 200, description = "Approved", body = RequestResponse)))]
#[post("/requests/{id}/approve")]
pub async fn approve(
	admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let request = state.requests.approve(&admin.id, &id).await?;
	Ok(HttpResponse::Ok().json(RequestResponse::from(&request)))
}

#[utoipa::path(tag = "requests", responses((status = 200, description = "Declined", body = RequestResponse)))]
#[post("/requests/{id}/decline")]
pub async fn decline(
	admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let request = state.requests.decline(&admin.id, &id).await?;
	Ok(HttpResponse::Ok().json(RequestResponse::from(&request)))
}

#[utoipa::path(tag = "requests", responses((status = 200, description = "Ranked releases for the request")))]
#[get("/requests/{id}/releases")]
pub async fn releases(
	_admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let releases = state.releases.for_request(&id).await?;
	Ok(HttpResponse::Ok().json(releases))
}

#[utoipa::path(tag = "requests", responses((status = 201, description = "Grab enqueued", body = DownloadResponse)))]
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
		info_hash: None,
	});

	let download = state.grabs.grab(&id, selection).await?;
	Ok(HttpResponse::Created().json(DownloadResponse::from(&download)))
}
