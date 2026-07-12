use actix_web::{HttpResponse, get, web};

use crate::dto::JobResponse;
use crate::error::ApiError;
use crate::extract::AdminUser;
use crate::pagination::{Page, PageParams, Pagination};
use crate::state::AppState;

#[utoipa::path(tag = "jobs", params(PageParams), responses((status = 200, body = Page<JobResponse>)))]
#[get("/jobs")]
pub async fn list(
	_admin: AdminUser,
	query: web::Query<PageParams>,
	state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
	let pagination = Pagination::resolve(query.page, query.limit);
	let jobs = state
		.jobs
		.list_page(pagination.limit, pagination.offset)
		.await?;
	let total = state.jobs.count().await?;
	let items: Vec<JobResponse> = jobs.iter().map(JobResponse::from).collect();
	Ok(HttpResponse::Ok().json(Page::new(items, &pagination, total)))
}
