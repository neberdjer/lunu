use actix_web::{HttpResponse, web};
use lunu_core::Error;
use lunu_core::services::ReleaseSelection;
use serde::Deserialize;

use crate::dto::{DownloadResponse, RequestResponse};
use crate::error::ApiError;
use crate::extract::{AdminUser, AuthUser};
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

pub async fn list(user: AuthUser, state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
	let requests = if user.role.is_admin() {
		state.requests.list().await?
	} else {
		state.requests.list_for_user(&user.id).await?
	};

	let response: Vec<RequestResponse> = requests.iter().map(RequestResponse::from).collect();
	Ok(HttpResponse::Ok().json(response))
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
	let request = state
		.requests
		.get(&id)
		.await?
		.filter(|request| user.role.is_admin() || request.user_id == user.id)
		.ok_or_else(|| Error::NotFound(format!("request {id}")))?;

	Ok(HttpResponse::Ok().json(RequestResponse::from(&request)))
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
