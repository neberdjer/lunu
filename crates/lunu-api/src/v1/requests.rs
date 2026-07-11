use std::str::FromStr;

use actix_web::{HttpResponse, web};
use lunu_core::models::RequestStatus;
use lunu_core::services::ReleaseSelection;
use serde::Deserialize;

use crate::dto::{ActivityResponse, DownloadResponse, RequestResponse};
use crate::error::ApiError;
use crate::extract::{AdminUser, AuthUser};
use crate::pagination::{Page, Pagination};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateRequestBody {
	asin: String,
}

#[derive(Deserialize, Default)]
pub struct GrabBody {
	download_url: Option<String>,
	#[serde(default)]
	title: String,
	#[serde(default)]
	indexer: String,
}

#[derive(Deserialize)]
pub struct RequestListParams {
	page: Option<i64>,
	limit: Option<i64>,
	status: Option<String>,
}

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

pub async fn create(
	user: AuthUser,
	state: web::Data<AppState>,
	body: web::Json<CreateRequestBody>,
) -> Result<HttpResponse, ApiError> {
	let request = state.requests.create(&user.0, &body.asin).await?;
	Ok(HttpResponse::Created().json(RequestResponse::from(&request)))
}

pub async fn get(
	user: AuthUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let id = id.into_inner();
	let request = state.requests.get_for(&user, &id).await?;
	Ok(HttpResponse::Ok().json(RequestResponse::from(&request)))
}

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

pub async fn approve(
	admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let request = state.requests.approve(&admin.id, &id).await?;
	Ok(HttpResponse::Ok().json(RequestResponse::from(&request)))
}

pub async fn decline(
	admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let request = state.requests.decline(&admin.id, &id).await?;
	Ok(HttpResponse::Ok().json(RequestResponse::from(&request)))
}

pub async fn releases(
	_admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let releases = state.releases.for_request(&id).await?;
	Ok(HttpResponse::Ok().json(releases))
}

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
