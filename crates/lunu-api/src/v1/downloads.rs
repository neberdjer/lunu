use actix_web::{HttpResponse, web};

use crate::dto::DownloadResponse;
use crate::error::ApiError;
use crate::extract::AdminUser;
use crate::state::AppState;

pub async fn list(_admin: AdminUser, state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
	let downloads = state.grabs.list().await?;
	let response: Vec<DownloadResponse> = downloads.iter().map(DownloadResponse::from).collect();
	Ok(HttpResponse::Ok().json(response))
}
