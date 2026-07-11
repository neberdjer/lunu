use actix_web::{HttpResponse, web};

use crate::dto::JobResponse;
use crate::error::ApiError;
use crate::extract::AdminUser;
use crate::state::AppState;

pub async fn list(_admin: AdminUser, state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
	let jobs = state.jobs.list().await?;
	let response: Vec<JobResponse> = jobs.iter().map(JobResponse::from).collect();
	Ok(HttpResponse::Ok().json(response))
}
