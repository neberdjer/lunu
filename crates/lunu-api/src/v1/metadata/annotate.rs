use std::collections::{HashMap, HashSet};

use lunu_core::models::{Book, ExternalId, RequestStatus};
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

pub(super) struct Presence {
	works: HashMap<ExternalId, String>,
	statuses: HashMap<String, RequestStatus>,
	available: HashSet<String>,
}

impl Presence {
	pub(super) async fn load(
		state: &AppState,
		user_id: &str,
		books: &[Book],
	) -> Result<Self, ApiError> {
		let ids: Vec<ExternalId> = books.iter().flat_map(|book| book.ids.clone()).collect();
		let asins: Vec<String> = books
			.iter()
			.filter_map(|book| book.asin().map(str::to_string))
			.collect();

		let works = state.works.resolve_ids(&ids).await?;
		let work_ids: Vec<String> = works.values().cloned().collect();

		let (statuses, available) = tokio::try_join!(
			state.requests.status_by_works(user_id, &work_ids),
			state.media.available_among(&asins),
		)?;

		Ok(Self {
			works,
			statuses,
			available,
		})
	}

	pub(super) fn status_for(&self, book: &Book) -> Option<&RequestStatus> {
		book.ids
			.iter()
			.find_map(|id| self.works.get(id))
			.and_then(|work_id| self.statuses.get(work_id))
	}

	pub(super) fn available(&self, book: &Book) -> bool {
		book.asin()
			.is_some_and(|asin| self.available.contains(asin))
	}
}

pub(super) async fn annotate(
	state: &AppState,
	user_id: &str,
	books: Vec<Book>,
) -> Result<Vec<SearchResult>, ApiError> {
	let presence = Presence::load(state, user_id, &books).await?;

	Ok(books
		.into_iter()
		.map(|book| SearchResult {
			request_status: presence
				.status_for(&book)
				.map(|status| status.as_str().to_string()),
			available: presence.available(&book),
			book: BookResponse::from(&book),
		})
		.collect())
}

pub(super) async fn annotated_detail(
	state: &AppState,
	user_id: &str,
	book: Book,
) -> Result<SearchResult, ApiError> {
	let mut results = annotate(state, user_id, vec![book]).await?;
	Ok(results.remove(0))
}
