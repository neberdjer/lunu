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
	#[serde(default)]
	page: Option<i64>,
}

#[derive(Serialize)]
struct SearchResult {
	#[serde(flatten)]
	book: Book,
	request_status: Option<&'static str>,
	available: bool,
}

#[utoipa::path(tag = "metadata", params(SearchQuery), responses((status = 200, description = "Search results, each with the caller request status")))]
#[get("/search")]
pub async fn search(
	user: AuthUser,
	state: web::Data<AppState>,
	query: web::Query<SearchQuery>,
) -> Result<HttpResponse, ApiError> {
	let books = state
		.metadata
		.search(&query.q, query.page.unwrap_or(1))
		.await?;
	let results = annotate(&state, &user.0.id, books).await?;
	Ok(HttpResponse::Ok().json(results))
}

async fn annotate(
	state: &AppState,
	user_id: &str,
	books: Vec<Book>,
) -> Result<Vec<SearchResult>, ApiError> {
	let asins: Vec<String> = books.iter().map(|book| book.asin.clone()).collect();
	let (statuses, available) = tokio::try_join!(
		state.requests.status_by_asin(user_id),
		state.media.available_among(&asins),
	)?;
	Ok(books
		.into_iter()
		.map(|book| SearchResult {
			request_status: statuses.get(&book.asin).map(|status| status.as_str()),
			available: available.contains(&book.asin),
			book,
		})
		.collect())
}

#[utoipa::path(tag = "metadata", responses((status = 200, description = "Similar books")))]
#[get("/books/{asin}/similar")]
pub async fn similar(
	user: AuthUser,
	state: web::Data<AppState>,
	asin: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let books = state.metadata.similar(&asin.into_inner()).await?;
	let results = annotate(&state, &user.0.id, books).await?;
	Ok(HttpResponse::Ok().json(results))
}

#[utoipa::path(tag = "metadata", responses((status = 200, description = "More books by the author")))]
#[get("/authors/{asin}/books")]
pub async fn author_books(
	user: AuthUser,
	state: web::Data<AppState>,
	asin: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let books = state.metadata.books_by_author(&asin.into_inner()).await?;
	let results = annotate(&state, &user.0.id, books).await?;
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
	let (status, media) = tokio::try_join!(
		state.requests.status_for_asin(&user.0.id, &asin),
		state.media.find(&asin),
	)?;
	Ok(HttpResponse::Ok().json(SearchResult {
		book,
		request_status: status.map(|status| status.as_str()),
		available: media.is_some(),
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
