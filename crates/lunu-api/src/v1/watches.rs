use actix_web::{HttpResponse, delete, get, post, web};
use serde::Deserialize;

use crate::dto::{RequestResponse, WatchResponse};
use crate::error::ApiError;
use crate::extract::AuthUser;
use crate::pagination::{Page, Pagination};
use crate::state::AppState;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateWatchBody {
	id: String,
}

#[derive(Deserialize, utoipa::IntoParams)]
pub struct WatchListParams {
	page: Option<i64>,
	limit: Option<i64>,
}

#[utoipa::path(tag = "watches", request_body = CreateWatchBody, responses((status = 201, description = "Book added to the watchlist", body = WatchResponse), (status = 409, description = "Already watched")))]
#[post("/watches")]
pub async fn create(
	user: AuthUser,
	state: web::Data<AppState>,
	body: web::Json<CreateWatchBody>,
) -> Result<HttpResponse, ApiError> {
	let id = crate::wire::parse_wire_id(&body.id)?;
	let watch = state.watches.create(&user.0, &id).await?;
	Ok(HttpResponse::Created().json(WatchResponse::from(&watch)))
}

#[utoipa::path(tag = "watches", params(WatchListParams), responses((status = 200, body = Page<WatchResponse>)))]
#[get("/watches")]
pub async fn list(
	user: AuthUser,
	state: web::Data<AppState>,
	query: web::Query<WatchListParams>,
) -> Result<HttpResponse, ApiError> {
	let pagination = Pagination::resolve(query.page, query.limit);
	let (items, total) = tokio::try_join!(
		state
			.watches
			.list_page(&user.id, pagination.limit, pagination.offset),
		state.watches.count(&user.id)
	)?;
	let items: Vec<WatchResponse> = items.iter().map(WatchResponse::from).collect();
	Ok(HttpResponse::Ok().json(Page::new(items, &pagination, total)))
}

#[utoipa::path(tag = "watches", params(("id" = String, Path, description = "Watch id")), responses((status = 204, description = "Removed from the watchlist"), (status = 404, description = "Unknown watch")))]
#[delete("/watches/{id}")]
pub async fn delete(
	user: AuthUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	state.watches.delete(&user.0, &id.into_inner()).await?;
	Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(tag = "watches", params(("id" = String, Path, description = "Watch id")), responses((status = 201, description = "Watch promoted to a request", body = RequestResponse), (status = 404, description = "Unknown watch")))]
#[post("/watches/{id}/request")]
pub async fn promote(
	user: AuthUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let request = state.watches.promote(&user.0, &id.into_inner()).await?;
	Ok(HttpResponse::Created().json(RequestResponse::from(&request)))
}
