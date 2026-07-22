use actix_web::{HttpResponse, get, web};

use crate::dto::ActivityResponse;
use crate::error::ApiError;
use crate::extract::AdminUser;
use crate::pagination::{Page, PageParams, Pagination};
use crate::state::AppState;

#[utoipa::path(tag = "activity", params(PageParams), responses((status = 200, body = Page<ActivityResponse>)))]
#[get("/activity")]
pub async fn list(
	_admin: AdminUser,
	query: web::Query<PageParams>,
	state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
	let pagination = Pagination::resolve(query.page, query.limit);
	let (activity, total) = tokio::try_join!(
		state
			.activity
			.list_page(pagination.limit, pagination.offset),
		state.activity.count()
	)?;

	let items: Vec<ActivityResponse> = activity.iter().map(ActivityResponse::from).collect();
	Ok(HttpResponse::Ok().json(Page::new(items, &pagination, total)))
}
