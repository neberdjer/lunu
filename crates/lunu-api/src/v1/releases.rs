use actix_web::{HttpResponse, get, web};
use serde::Deserialize;

use crate::error::ApiError;
use crate::extract::AdminUser;
use crate::state::AppState;

#[derive(Deserialize, utoipa::IntoParams)]
pub struct ReleaseSearchQuery {
	q: String,
}

#[utoipa::path(tag = "releases", params(ReleaseSearchQuery), responses((status = 200, description = "Ranked torrent releases from the indexer for a free-text query")))]
#[get("/releases/search")]
pub async fn search(
	_admin: AdminUser,
	state: web::Data<AppState>,
	query: web::Query<ReleaseSearchQuery>,
) -> Result<HttpResponse, ApiError> {
	let releases = state.releases.search(&query.q).await?;
	Ok(HttpResponse::Ok().json(releases))
}
