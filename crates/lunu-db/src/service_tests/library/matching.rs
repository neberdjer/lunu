use lunu_core::models::MatchedBy;

use super::*;

fn found(title: &str, asin: &str, author: &str) -> Book {
	Book {
		ids: vec![ExternalId::asin(asin)],
		authors: vec![author.to_string()],
		..book(title)
	}
}

fn stub(id: &'static str, books: Option<Vec<Book>>) -> Arc<SearchStub> {
	Arc::new(SearchStub {
		id,
		books,
		book: None,
	})
}

async fn media_for(db: &Db, abs_id: &str) -> lunu_core::models::Media {
	SqlxMediaRepo::new(db.clone())
		.find_by_abs_item_id(abs_id)
		.await
		.unwrap()
		.unwrap()
}

#[tokio::test]
async fn exact_title_and_author_match_links_the_work() {
	let db = memory_db().await;
	let books = vec![found("Book b", "B-FOUND", "Isaac Asimov")];
	let service = library_service_with(&db, vec![item("b", None)], stub("finder", Some(books)));

	let summary = service.sync().await.unwrap();
	assert_eq!(summary.imported, 1);
	assert_eq!(summary.matched, 1);

	let media = media_for(&db, "b").await;
	assert_eq!(media.asin.as_deref(), Some("B-FOUND"));
	assert_eq!(media.matched_by, Some(MatchedBy::Title));
	assert!(media.work_id.is_some());
	assert_eq!(media.title, "Book b");
	assert!(!media.overridden);
}

#[tokio::test]
async fn a_close_title_matches_fuzzily_above_the_floor() {
	let db = memory_db().await;
	let books = vec![found("Book bb", "B-FUZZY", "Isaac Asimov")];
	let service = library_service_with(&db, vec![item("b", None)], stub("finder", Some(books)));

	let summary = service.sync().await.unwrap();
	assert_eq!(summary.matched, 1);

	let media = media_for(&db, "b").await;
	assert_eq!(media.asin.as_deref(), Some("B-FUZZY"));
	assert_eq!(media.matched_by, Some(MatchedBy::Fuzzy));
}

#[tokio::test]
async fn a_different_title_stays_unmatched() {
	let db = memory_db().await;
	let books = vec![found(
		"Completely Different Saga",
		"B-WRONG",
		"Isaac Asimov",
	)];
	let service = library_service_with(&db, vec![item("b", None)], stub("finder", Some(books)));

	let summary = service.sync().await.unwrap();
	assert_eq!(summary.matched, 0);

	let media = media_for(&db, "b").await;
	assert_eq!(media.asin, None);
	assert_eq!(media.matched_by, None);
}

#[tokio::test]
async fn an_author_mismatch_stays_unmatched_even_on_exact_title() {
	let db = memory_db().await;
	let books = vec![found("Book b", "B-WRONG", "Somebody Else")];
	let service = library_service_with(&db, vec![item("b", None)], stub("finder", Some(books)));

	service.sync().await.unwrap();

	let media = media_for(&db, "b").await;
	assert_eq!(media.asin, None);
	assert_eq!(media.matched_by, None);
}

#[tokio::test]
async fn a_provider_error_never_fails_the_sync() {
	let db = memory_db().await;
	let service = library_service_with(&db, vec![item("b", None)], stub("finder", None));

	let summary = service.sync().await.unwrap();
	assert_eq!(summary.imported, 1);
	assert_eq!(summary.matched, 0);

	let media = media_for(&db, "b").await;
	assert_eq!(media.asin, None);
}

#[tokio::test]
async fn resync_preserves_an_earlier_search_match() {
	let db = memory_db().await;
	let books = vec![found("Book b", "B-FOUND", "Isaac Asimov")];
	library_service_with(&db, vec![item("b", None)], stub("finder", Some(books)))
		.sync()
		.await
		.unwrap();

	let resynced =
		library_service_with(&db, vec![item("b", None)], stub("empty", Some(Vec::new())));
	let summary = resynced.sync().await.unwrap();
	assert_eq!(summary.matched, 0);
	assert_eq!(summary.updated, 0);
	assert_eq!(summary.skipped, 1);

	let media = media_for(&db, "b").await;
	assert_eq!(media.asin.as_deref(), Some("B-FOUND"));
	assert_eq!(media.matched_by, Some(MatchedBy::Title));
}

#[tokio::test]
async fn an_item_that_gains_its_own_asin_outranks_a_search_match() {
	let db = memory_db().await;
	let books = vec![found("Book b", "B-FOUND", "Isaac Asimov")];
	library_service_with(&db, vec![item("b", None)], stub("finder", Some(books)))
		.sync()
		.await
		.unwrap();

	let resynced = library_service_with(
		&db,
		vec![item("b", Some("B-REAL"))],
		stub("empty", Some(Vec::new())),
	);
	let summary = resynced.sync().await.unwrap();
	assert_eq!(summary.updated, 1);

	let media = media_for(&db, "b").await;
	assert_eq!(media.asin.as_deref(), Some("B-REAL"));
	assert_eq!(media.matched_by, Some(MatchedBy::Asin));
}
