use lunu_core::models::MatchedBy;

use super::*;

fn found(title: &str, asin: &str, author: &str) -> Book {
	Book {
		ids: vec![ExternalId::asin(asin)],
		authors: vec![author.to_string()],
		..book(title)
	}
}

fn in_series(title: &str, asin: &str, author: &str, series: &str, position: &str) -> Book {
	Book {
		series: vec![SeriesRef {
			name: series.to_string(),
			position: Some(position.to_string()),
			asin: None,
		}],
		..found(title, asin, author)
	}
}

fn shelved_at(abs_id: &str, position: &str) -> LibraryItem {
	LibraryItem {
		series_sequence: Some(position.to_string()),
		..item(abs_id, None)
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
async fn a_series_position_matches_an_item_the_shelf_titled_differently() {
	let db = memory_db().await;
	let books = vec![in_series(
		"Foundation",
		"B-SERIES",
		"Isaac Asimov",
		"Foundation",
		"1",
	)];
	let service =
		library_service_with(&db, vec![shelved_at("b", "1")], stub("finder", Some(books)));

	let summary = service.sync().await.unwrap();
	assert_eq!(summary.matched, 1);

	let media = media_for(&db, "b").await;
	assert_eq!(
		media.matched_by,
		Some(MatchedBy::Series),
		"a shelf title no search result resembles still matches on series position"
	);
	assert_eq!(media.asin.as_deref(), Some("B-SERIES"));
	assert!(media.work_id.is_some());
}

async fn work_known_by_isbn(db: &Db, isbn: &str) -> String {
	let mut known = book("Foundation");
	known.ids = vec![ExternalId::isbn(isbn)];
	work_service(db).for_book(&known).await.unwrap().unwrap()
}

#[tokio::test]
async fn an_isbn_links_to_a_work_already_known_by_that_isbn() {
	let db = memory_db().await;
	let isbn = "9780553293357";
	let work_id = work_known_by_isbn(&db, isbn).await;

	let service = library_service_with(
		&db,
		vec![item_with_isbn("b", isbn)],
		stub("empty", Some(Vec::new())),
	);
	let summary = service.sync().await.unwrap();
	assert_eq!(summary.imported, 1);

	let media = media_for(&db, "b").await;
	assert_eq!(media.matched_by, Some(MatchedBy::Isbn));
	assert_eq!(
		media.work_id.as_deref(),
		Some(work_id.as_str()),
		"the abs item joins the work a request already established by isbn"
	);
	assert_eq!(media.asin, None);
}

#[tokio::test]
async fn an_isbn_unknown_to_any_work_falls_through_to_search() {
	let db = memory_db().await;
	let service = library_service_with(
		&db,
		vec![item_with_isbn("b", "9780000000000")],
		stub("empty", Some(Vec::new())),
	);
	service.sync().await.unwrap();

	let media = media_for(&db, "b").await;
	assert_eq!(
		media.matched_by, None,
		"an isbn no work carries is not enough to invent a match"
	);
	assert_eq!(media.work_id, None);
}

#[tokio::test]
async fn a_known_isbn_outranks_an_earlier_fuzzy_search_match() {
	let db = memory_db().await;
	let isbn = "9780553293357";
	let books = vec![found("Book b", "B-FOUND", "Isaac Asimov")];
	library_service_with(&db, vec![item("b", None)], stub("finder", Some(books)))
		.sync()
		.await
		.unwrap();

	let work_id = work_known_by_isbn(&db, isbn).await;
	let resynced = library_service_with(
		&db,
		vec![item_with_isbn("b", isbn)],
		stub("empty", Some(Vec::new())),
	);
	assert_eq!(resynced.sync().await.unwrap().updated, 1);

	let media = media_for(&db, "b").await;
	assert_eq!(media.matched_by, Some(MatchedBy::Isbn));
	assert_eq!(media.work_id.as_deref(), Some(work_id.as_str()));
	assert_eq!(
		media.asin, None,
		"the exact isbn match clears the earlier fuzzy search asin"
	);
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
