use actix_web::{HttpResponse, get, web};
use lunu_core::Error;
use lunu_core::models::Book;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::extract::AuthUser;
use crate::state::AppState;

#[derive(Deserialize, utoipa::IntoParams)]
pub struct SearchQuery {
	q: String,
}

#[derive(Serialize)]
struct SearchResult {
	#[serde(flatten)]
	book: Book,
	request_status: Option<&'static str>,
}

#[utoipa::path(tag = "metadata", params(SearchQuery), responses((status = 200, description = "Search results, each with the caller request status")))]
#[get("/search")]
pub async fn search(
	user: AuthUser,
	state: web::Data<AppState>,
	query: web::Query<SearchQuery>,
) -> Result<HttpResponse, ApiError> {
	let books = state.metadata.search(&query.q).await?;
	let statuses = state.requests.status_by_asin(&user.0.id).await?;
	let results: Vec<SearchResult> = books
		.into_iter()
		.map(|book| SearchResult {
			request_status: statuses.get(&book.asin).map(|status| status.as_str()),
			book,
		})
		.collect();
	Ok(HttpResponse::Ok().json(results))
}

#[utoipa::path(tag = "metadata", responses((status = 200, description = "Book detail with request status"), (status = 404, description = "Unknown ASIN")))]
#[get("/books/{asin}")]
pub async fn book_detail(
	user: AuthUser,
	state: web::Data<AppState>,
	asin: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let asin = asin.into_inner();
	let book = state
		.metadata
		.get_book(&asin)
		.await?
		.ok_or_else(|| Error::NotFound(format!("book {asin}")))?;
	let request_status = state
		.requests
		.status_for_asin(&user.0.id, &asin)
		.await?
		.map(|status| status.as_str());
	Ok(HttpResponse::Ok().json(SearchResult {
		book,
		request_status,
	}))
}

#[utoipa::path(tag = "metadata", responses((status = 200, description = "Chapter list"), (status = 404, description = "No chapters")))]
#[get("/books/{asin}/chapters")]
pub async fn chapters(
	_user: AuthUser,
	state: web::Data<AppState>,
	asin: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let asin = asin.into_inner();
	let chapters = state
		.metadata
		.get_chapters(&asin)
		.await?
		.ok_or_else(|| Error::NotFound(format!("chapters {asin}")))?;
	Ok(HttpResponse::Ok().json(chapters))
}
