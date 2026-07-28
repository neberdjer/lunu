use actix_web::{HttpResponse, get, post, web};
use lunu_core::Error;
use lunu_core::models::{Book, ExternalId};
use lunu_core::services::NewRequest;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::extract::AuthUser;
use crate::pagination::{Page, Pagination};
use crate::state::AppState;

mod annotate;

use annotate::{Presence, SearchResult, annotate, annotated_detail};

const OPAQUE_BOOK_ID: &str = "Opaque book identifier, taken from a search result. Treat it as \
	meaningless text: do not parse it, construct it, or assume it stays an ASIN.";
const OPAQUE_AUTHOR_ID: &str = "Opaque author identifier, taken from a book's author_asins. Treat \
	it as meaningless text: do not parse it, construct it, or assume it stays an ASIN.";

fn enforce_metadata_rate_limit(state: &AppState, user_id: &str) -> Result<(), ApiError> {
	if state.metadata_rate_limiter.check(user_id) {
		Ok(())
	} else {
		Err(Error::RateLimited.into())
	}
}

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

#[derive(Serialize, utoipa::ToSchema)]
struct FailedRequest {
	asin: String,
	error: String,
}

#[derive(Serialize, utoipa::ToSchema)]
struct SeriesRequestResult {
	requested: Vec<String>,
	already_present: usize,
	failed: Vec<FailedRequest>,
	truncated: bool,
}

#[utoipa::path(tag = "metadata", params(SearchQuery), responses((status = 200, description = "Search results, each with the caller request status", body = Vec<SearchResult>)))]
#[get("/search")]
pub async fn search(
	user: AuthUser,
	state: web::Data<AppState>,
	query: web::Query<SearchQuery>,
) -> Result<HttpResponse, ApiError> {
	enforce_metadata_rate_limit(&state, &user.0.id)?;
	let books = state
		.metadata
		.search(&query.q, query.page.unwrap_or(1))
		.await?;
	let results = annotate(&state, &user.0.id, books).await?;
	Ok(HttpResponse::Ok().json(results))
}

#[utoipa::path(tag = "metadata", params(SeriesSearchQuery), responses((status = 200, description = "Matching series (name and asin)")))]
#[get("/series")]
pub async fn series_search(
	user: AuthUser,
	state: web::Data<AppState>,
	query: web::Query<SeriesSearchQuery>,
) -> Result<HttpResponse, ApiError> {
	enforce_metadata_rate_limit(&state, &user.0.id)?;
	let series = state.metadata.search_series(&query.q).await?;
	Ok(HttpResponse::Ok().json(series))
}

#[utoipa::path(tag = "metadata", params(SeriesBooksQuery), responses((status = 200, description = "Paginated books in the series, ordered by position, each with caller request status", body = Page<SearchResult>)))]
#[get("/series/books")]
pub async fn series_books(
	user: AuthUser,
	state: web::Data<AppState>,
	query: web::Query<SeriesBooksQuery>,
) -> Result<HttpResponse, ApiError> {
	enforce_metadata_rate_limit(&state, &user.0.id)?;
	let series = query.asin.as_deref().map(ExternalId::asin);
	let all = state
		.metadata
		.series_books(&query.name, series.as_ref())
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

#[utoipa::path(tag = "metadata", request_body = SeriesRequestBody, responses((status = 200, description = "Requests the identifiable, not-yet-owned or requested books in the series (best-effort discovery)", body = SeriesRequestResult)))]
#[post("/series/request")]
pub async fn series_request(
	user: AuthUser,
	state: web::Data<AppState>,
	body: web::Json<SeriesRequestBody>,
) -> Result<HttpResponse, ApiError> {
	enforce_metadata_rate_limit(&state, &user.0.id)?;
	let body = body.into_inner();
	if let Some(profile_id) = body.quality_profile_id.as_deref() {
		state.quality_profiles.require(profile_id).await?;
	}

	let series = body.asin.as_deref().map(ExternalId::asin);
	let books = state
		.metadata
		.series_books(&body.name, series.as_ref())
		.await?;
	let presence = Presence::load(&state, &user.0.id, &books).await?;

	let mut requested = Vec::new();
	let mut already_present = 0;
	let mut failed = Vec::new();
	let mut truncated = false;
	for book in books {
		let Some(asin) = book.asin().map(str::to_string) else {
			continue;
		};
		if presence.status_for(&book).is_some() || presence.available(&book) {
			already_present += 1;
			continue;
		}
		if requested.len() + failed.len() >= crate::v1::requests::MAX_BULK_ITEMS {
			truncated = true;
			break;
		}
		let input = NewRequest {
			id: ExternalId::asin(&asin),
			notes: body.notes.clone(),
			quality_profile_id: body.quality_profile_id.clone(),
		};
		match state.requests.create_with_book(&user.0, input, book).await {
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
		truncated,
	}))
}

#[utoipa::path(tag = "metadata", params(("id" = String, Path, description = OPAQUE_BOOK_ID)), responses((status = 200, description = "Similar books", body = Vec<SearchResult>)))]
#[get("/books/{id}/similar")]
pub async fn similar(
	user: AuthUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	enforce_metadata_rate_limit(&state, &user.0.id)?;
	let books = state
		.metadata
		.similar(&crate::wire::parse_wire_id(&id)?)
		.await?;
	let results = annotate(&state, &user.0.id, books).await?;
	Ok(HttpResponse::Ok().json(results))
}

#[utoipa::path(tag = "metadata", params(("id" = String, Path, description = OPAQUE_AUTHOR_ID)), responses((status = 200, description = "More books by the author", body = Vec<SearchResult>)))]
#[get("/authors/{id}/books")]
pub async fn author_books(
	user: AuthUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	enforce_metadata_rate_limit(&state, &user.0.id)?;
	let books = state
		.metadata
		.books_by_author(&crate::wire::parse_wire_id(&id)?)
		.await?;
	let results = annotate(&state, &user.0.id, books).await?;
	Ok(HttpResponse::Ok().json(results))
}

#[utoipa::path(tag = "metadata", params(("id" = String, Path, description = OPAQUE_BOOK_ID)), responses((status = 200, description = "Book detail with request status", body = SearchResult), (status = 404, description = "Unknown book")))]
#[get("/books/{id}")]
pub async fn book_detail(
	user: AuthUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	enforce_metadata_rate_limit(&state, &user.0.id)?;
	let wire = id.into_inner();
	let book = state
		.metadata
		.get_book(&crate::wire::parse_wire_id(&wire)?)
		.await?
		.ok_or_else(|| Error::NotFound(format!("book {wire}")))?;
	let result = annotated_detail(&state, &user.0.id, book).await?;
	Ok(HttpResponse::Ok().json(result))
}

#[utoipa::path(tag = "metadata", params(("id" = String, Path, description = OPAQUE_BOOK_ID)), responses((status = 200, description = "Freshly fetched book detail, bypassing the cache", body = SearchResult), (status = 404, description = "Unknown book")))]
#[post("/books/{id}/refresh")]
pub async fn refresh_book(
	user: AuthUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	enforce_metadata_rate_limit(&state, &user.0.id)?;
	let wire = id.into_inner();
	let book = state
		.metadata
		.refresh_book(&crate::wire::parse_wire_id(&wire)?)
		.await?
		.ok_or_else(|| Error::NotFound(format!("book {wire}")))?;
	let result = annotated_detail(&state, &user.0.id, book).await?;
	Ok(HttpResponse::Ok().json(result))
}

#[utoipa::path(tag = "metadata", params(("id" = String, Path, description = OPAQUE_BOOK_ID)), responses((status = 200, description = "Chapter list"), (status = 404, description = "No chapters")))]
#[get("/books/{id}/chapters")]
pub async fn chapters(
	user: AuthUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	enforce_metadata_rate_limit(&state, &user.0.id)?;
	let wire = id.into_inner();
	let chapters = state
		.metadata
		.get_chapters(&crate::wire::parse_wire_id(&wire)?)
		.await?
		.ok_or_else(|| Error::NotFound(format!("chapters {wire}")))?;
	Ok(HttpResponse::Ok().json(chapters))
}
