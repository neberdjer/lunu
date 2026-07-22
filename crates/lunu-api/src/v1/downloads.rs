use actix_web::{HttpResponse, get, web};

use crate::dto::DownloadResponse;
use crate::error::ApiError;
use crate::extract::AdminUser;
use crate::pagination::{Page, PageParams, Pagination};
use crate::state::AppState;

#[utoipa::path(tag = "downloads", params(PageParams), responses((status = 200, body = Page<DownloadResponse>)))]
#[get("/downloads")]
pub async fn list(
	_admin: AdminUser,
	query: web::Query<PageParams>,
	state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
	let pagination = Pagination::resolve(query.page, query.limit);
	let (downloads, total) = tokio::try_join!(
		state.grabs.list_page(pagination.limit, pagination.offset),
		state.grabs.count()
	)?;
	let items: Vec<DownloadResponse> = downloads.iter().map(DownloadResponse::from).collect();
	Ok(HttpResponse::Ok().json(Page::new(items, &pagination, total)))
}
