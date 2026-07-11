use actix_web::{HttpResponse, web};

use crate::dto::DownloadResponse;
use crate::error::ApiError;
use crate::extract::AdminUser;
use crate::pagination::{Page, PageParams, Pagination};
use crate::state::AppState;

pub async fn list(
	_admin: AdminUser,
	query: web::Query<PageParams>,
	state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
	let pagination = Pagination::resolve(query.page, query.limit);
	let downloads = state
		.grabs
		.list_page(pagination.limit, pagination.offset)
		.await?;
	let total = state.grabs.count().await?;
	let items: Vec<DownloadResponse> = downloads.iter().map(DownloadResponse::from).collect();
	Ok(HttpResponse::Ok().json(Page::new(items, &pagination, total)))
}
