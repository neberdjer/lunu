use actix_web::{HttpResponse, get, post, web};
use lunu_core::Error;
use serde::Serialize;

use crate::dto::NotificationResponse;
use crate::error::ApiError;
use crate::extract::AuthUser;
use crate::pagination::{Page, PageParams, Pagination};
use crate::state::AppState;

#[derive(Serialize, utoipa::ToSchema)]
pub struct UnreadCount {
	pub unread: i64,
}

#[utoipa::path(tag = "notifications", params(PageParams), responses((status = 200, body = Page<NotificationResponse>)))]
#[get("/notifications")]
pub async fn list(
	user: AuthUser,
	query: web::Query<PageParams>,
	state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
	let pagination = Pagination::resolve(query.page, query.limit);
	let (notifications, total) = tokio::try_join!(
		state
			.inbox
			.list(&user.0.id, pagination.limit, pagination.offset),
		state.inbox.count(&user.0.id),
	)?;

	let items: Vec<NotificationResponse> = notifications
		.iter()
		.map(NotificationResponse::from)
		.collect();
	Ok(HttpResponse::Ok().json(Page::new(items, &pagination, total)))
}

#[utoipa::path(tag = "notifications", responses((status = 200, body = UnreadCount)))]
#[get("/notifications/unread-count")]
pub async fn unread_count(
	user: AuthUser,
	state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
	let unread = state.inbox.unread_count(&user.0.id).await?;
	Ok(HttpResponse::Ok().json(UnreadCount { unread }))
}

#[utoipa::path(tag = "notifications", responses((status = 204, description = "Marked read"), (status = 404, description = "Not found")))]
#[post("/notifications/{id}/read")]
pub async fn mark_read(
	user: AuthUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let id = id.into_inner();
	if !state.inbox.mark_read(&user.0.id, &id).await? {
		return Err(Error::NotFound(format!("notification {id}")).into());
	}
	Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(tag = "notifications", responses((status = 204, description = "All marked read")))]
#[post("/notifications/read-all")]
pub async fn mark_all_read(
	user: AuthUser,
	state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
	state.inbox.mark_all_read(&user.0.id).await?;
	Ok(HttpResponse::NoContent().finish())
}
