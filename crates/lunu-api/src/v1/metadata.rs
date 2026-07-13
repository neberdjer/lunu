use std::collections::{HashMap, HashSet};

use actix_web::{HttpResponse, get, post, web};
use lunu_core::Error;
use lunu_core::models::{Book, RequestStatus};
use lunu_core::services::NewRequest;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::extract::AuthUser;
use crate::pagination::{Page, Pagination};
use crate::state::AppState;

#[derive(Deserialize, utoipa::IntoParams)]
pub struct SearchQuery {
	q: String,
	#[serde(default)]
	page: Option<i64>,
}

#[derive(Deserialize, utoipa::IntoParams)]
pub struct SeriesSearchQuery {
	q: String,
}

#[derive(Deserialize, utoipa::IntoParams)]
pub struct SeriesBooksQuery {
	name: String,
	#[serde(default)]
	asin: Option<String>,
	#[serde(default)]
	page: Option<i64>,
	#[serde(default)]
	limit: Option<i64>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct SeriesRequestBody {
	name: String,
	#[serde(default)]
	asin: Option<String>,
	#[serde(default)]
	notes: Option<String>,
	#[serde(default)]
	quality_profile_id: Option<String>,
}

#[derive(Serialize)]
struct FailedRequest {
	asin: String,
	error: String,
}

#[derive(Serialize)]
struct SeriesRequestResult {
	requested: Vec<String>,
	already_present: usize,
	failed: Vec<FailedRequest>,
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

async fn presence(
	state: &AppState,
	user_id: &str,
	asins: &[String],
) -> Result<(HashMap<String, RequestStatus>, HashSet<String>), ApiError> {
	Ok(tokio::try_join!(
		state.requests.status_by_asin(user_id),
		state.media.available_among(asins),
	)?)
}

async fn annotate(
	state: &AppState,
	user_id: &str,
	books: Vec<Book>,
) -> Result<Vec<SearchResult>, ApiError> {
	let asins: Vec<String> = books.iter().map(|book| book.asin.clone()).collect();
	let (statuses, available) = presence(state, user_id, &asins).await?;
	Ok(books
		.into_iter()
		.map(|book| SearchResult {
			request_status: statuses.get(&book.asin).map(|status| status.as_str()),
			available: available.contains(&book.asin),
			book,
		})
		.collect())
}

#[utoipa::path(tag = "metadata", params(SeriesSearchQuery), responses((status = 200, description = "Matching series (name and asin)")))]
#[get("/series")]
pub async fn series_search(
	_user: AuthUser,
	state: web::Data<AppState>,
	query: web::Query<SeriesSearchQuery>,
) -> Result<HttpResponse, ApiError> {
	let series = state.metadata.search_series(&query.q).await?;
	Ok(HttpResponse::Ok().json(series))
}

#[utoipa::path(tag = "metadata", params(SeriesBooksQuery), responses((status = 200, description = "Paginated books in the series, ordered by position, each with caller request status")))]
#[get("/series/books")]
pub async fn series_books(
	user: AuthUser,
	state: web::Data<AppState>,
	query: web::Query<SeriesBooksQuery>,
) -> Result<HttpResponse, ApiError> {
	let all = state
		.metadata
		.series_books(&query.name, query.asin.as_deref())
		.await?;

	let pagination = Pagination::resolve(query.page, query.limit);
	let total = all.len() as i64;
	let page: Vec<Book> = all
		.into_iter()
		.skip(pagination.offset.max(0) as usize)
		.take(pagination.limit as usize)
		.collect();

	let items = annotate(&state, &user.0.id, page).await?;
	Ok(HttpResponse::Ok().json(Page::new(items, &pagination, total)))
}

#[utoipa::path(tag = "metadata", responses((status = 200, description = "Requests every not-yet-owned or requested book in the series")))]
#[post("/series/request")]
pub async fn series_request(
	user: AuthUser,
	state: web::Data<AppState>,
	body: web::Json<SeriesRequestBody>,
) -> Result<HttpResponse, ApiError> {
	let body = body.into_inner();
	if let Some(profile_id) = body.quality_profile_id.as_deref() {
		state.quality_profiles.require(profile_id).await?;
	}

	let books = state
		.metadata
		.series_books(&body.name, body.asin.as_deref())
		.await?;
	let asins: Vec<String> = books.iter().map(|book| book.asin.clone()).collect();
	let (statuses, available) = presence(&state, &user.0.id, &asins).await?;

	let mut requested = Vec::new();
	let mut already_present = 0;
	let mut failed = Vec::new();
	for asin in asins {
		if statuses.contains_key(&asin) || available.contains(&asin) {
			already_present += 1;
			continue;
		}
		let input = NewRequest {
			asin: asin.clone(),
			notes: body.notes.clone(),
			quality_profile_id: body.quality_profile_id.clone(),
		};
		match state.requests.create(&user.0, input).await {
			Ok(_) => requested.push(asin),
			Err(error) => failed.push(FailedRequest {
				asin,
				error: error.code().to_string(),
			}),
		}
	}

	Ok(HttpResponse::Ok().json(SeriesRequestResult {
		requested,
		already_present,
		failed,
	}))
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
