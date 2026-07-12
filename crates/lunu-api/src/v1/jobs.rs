use std::str::FromStr;

use actix_web::{HttpResponse, delete, get, post, web};
use lunu_core::models::JobStatus;
use serde::Deserialize;

use crate::dto::JobResponse;
use crate::error::ApiError;
use crate::extract::AdminUser;
use crate::pagination::{Page, Pagination};
use crate::state::AppState;

#[derive(Deserialize, utoipa::IntoParams)]
pub struct JobListParams {
	page: Option<i64>,
	limit: Option<i64>,
	status: Option<String>,
}

#[utoipa::path(tag = "jobs", params(JobListParams), responses((status = 200, body = Page<JobResponse>)))]
#[get("/jobs")]
pub async fn list(
	_admin: AdminUser,
	query: web::Query<JobListParams>,
	state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
	let pagination = Pagination::resolve(query.page, query.limit);
	let status = query
		.status
		.as_deref()
		.map(JobStatus::from_str)
		.transpose()?;
	let status = status.as_ref().map(JobStatus::as_str);

	let (jobs, total) = tokio::try_join!(
		state
			.jobs
			.list_page(status, pagination.limit, pagination.offset),
		state.jobs.count(status),
	)?;
	let items: Vec<JobResponse> = jobs.iter().map(JobResponse::from).collect();
	Ok(HttpResponse::Ok().json(Page::new(items, &pagination, total)))
}

#[utoipa::path(tag = "jobs", responses((status = 204, description = "Job requeued"), (status = 409, description = "Only a failed job can be requeued")))]
#[post("/jobs/{id}/retry")]
pub async fn retry(
	_admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	state.jobs.requeue(&id.into_inner()).await?;
	Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(tag = "jobs", responses((status = 204, description = "Job cancelled")))]
#[delete("/jobs/{id}")]
pub async fn cancel(
	_admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	state.jobs.cancel(&id.into_inner()).await?;
	Ok(HttpResponse::NoContent().finish())
}
