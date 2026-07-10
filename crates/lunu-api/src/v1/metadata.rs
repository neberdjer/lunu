use actix_web::{HttpResponse, web};
use lunu_core::Error;
use serde::Deserialize;

use crate::error::ApiError;
use crate::extract::AuthUser;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct SearchQuery {
	q: String,
}

pub async fn search(
	_user: AuthUser,
	state: web::Data<AppState>,
	query: web::Query<SearchQuery>,
) -> Result<HttpResponse, ApiError> {
	let books = state.metadata.search(&query.q).await?;
	Ok(HttpResponse::Ok().json(books))
}

pub async fn book(
	_user: AuthUser,
	state: web::Data<AppState>,
	asin: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let asin = asin.into_inner();
	let book = state
		.metadata
		.get_book(&asin)
		.await?
		.ok_or_else(|| Error::NotFound(format!("book {asin}")))?;
	Ok(HttpResponse::Ok().json(book))
}

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
