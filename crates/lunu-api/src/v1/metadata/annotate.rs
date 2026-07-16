use lunu_core::models::{Book, ExternalId};
use serde::Serialize;

use crate::dto::BookResponse;
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Serialize, utoipa::ToSchema)]
pub(super) struct SearchResult {
	#[serde(flatten)]
	pub(super) book: BookResponse,
	pub(super) request_status: Option<String>,
	pub(super) available: bool,
}

pub(super) async fn annotate(
	state: &AppState,
	user_id: &str,
	books: Vec<Book>,
) -> Result<Vec<SearchResult>, ApiError> {
	let ids: Vec<ExternalId> = books.iter().flat_map(|book| book.ids.clone()).collect();
	let asins: Vec<String> = books
		.iter()
		.filter_map(|book| book.asin().map(str::to_string))
		.collect();

	let works = state.works.resolve_ids(&ids).await?;
	let work_ids: Vec<String> = {
		let mut seen: Vec<String> = works.values().cloned().collect();
		seen.sort();
		seen.dedup();
		seen
	};

	let (statuses, available) = tokio::try_join!(
		state.requests.status_by_works(user_id, &work_ids),
		state.media.available_among(&asins),
	)?;

	Ok(books
		.into_iter()
		.map(|book| {
			let status = book
				.ids
				.iter()
				.find_map(|id| works.get(id))
				.and_then(|work_id| statuses.get(work_id));
			SearchResult {
				request_status: status.map(|status| status.as_str().to_string()),
				available: book.asin().is_some_and(|asin| available.contains(asin)),
				book: BookResponse::from(&book),
			}
		})
		.collect())
}
