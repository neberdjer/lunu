use lunu_core::models::{ExternalId, IdScheme, Work};
use lunu_core::repo::WorkRepo;

use crate::convert::format_dt;

use super::*;

fn work(title: &str) -> Work {
	Work {
		id: format!("work-{title}"),
		title: title.to_string(),
		author: Some("J.R.R. Tolkien".to_string()),
		cover_url: Some("cover".to_string()),
		created_at: Utc::now(),
	}
}

fn repo(db: &Db) -> SqlxWorkRepo {
	SqlxWorkRepo::new(db.clone())
}

#[tokio::test]
async fn a_work_round_trips() {
	let db = memory_db().await;
	let repo = repo(&db);
	let hobbit = work("The Hobbit");
	repo.insert(&hobbit).await.unwrap();

	let found = repo.find_by_id(&hobbit.id).await.unwrap().unwrap();
	assert_eq!(found.id, hobbit.id);
	assert_eq!(found.title, hobbit.title);
	assert_eq!(found.author, hobbit.author);
	assert_eq!(found.cover_url, hobbit.cover_url);
	assert_eq!(
		found.created_at.timestamp_micros(),
		hobbit.created_at.timestamp_micros(),
		"timestamps are stored to microseconds"
	);
	assert!(repo.find_by_id("nope").await.unwrap().is_none());
}

#[tokio::test]
async fn a_work_is_found_by_any_of_its_external_ids() {
	let db = memory_db().await;
	let repo = repo(&db);
	let hobbit = work("The Hobbit");
	repo.insert(&hobbit).await.unwrap();
	repo.link_external_id(&hobbit.id, &ExternalId::asin("1705009050"))
		.await
		.unwrap();
	repo.link_external_id(&hobbit.id, &ExternalId::isbn("9781705009055"))
		.await
		.unwrap();

	for id in [
		ExternalId::asin("1705009050"),
		ExternalId::isbn("9781705009055"),
	] {
		assert_eq!(
			repo.find_by_external_id(&id).await.unwrap().as_deref(),
			Some(hobbit.id.as_str()),
			"{id} must resolve to the same work"
		);
	}
}

#[tokio::test]
async fn the_same_value_under_two_schemes_is_two_different_books() {
	let db = memory_db().await;
	let repo = repo(&db);
	let one = work("One");
	let two = work("Two");
	repo.insert(&one).await.unwrap();
	repo.insert(&two).await.unwrap();

	repo.link_external_id(&one.id, &ExternalId::asin("9780007487295"))
		.await
		.unwrap();
	repo.link_external_id(&two.id, &ExternalId::isbn("9780007487295"))
		.await
		.unwrap();

	assert_eq!(
		repo.find_by_external_id(&ExternalId::asin("9780007487295"))
			.await
			.unwrap(),
		Some(one.id)
	);
	assert_eq!(
		repo.find_by_external_id(&ExternalId::isbn("9780007487295"))
			.await
			.unwrap(),
		Some(two.id)
	);
}

#[tokio::test]
async fn one_external_id_cannot_name_two_works() {
	let db = memory_db().await;
	let repo = repo(&db);
	let one = work("One");
	let two = work("Two");
	repo.insert(&one).await.unwrap();
	repo.insert(&two).await.unwrap();

	let asin = ExternalId::asin("1705009050");
	repo.link_external_id(&one.id, &asin).await.unwrap();
	let clash = repo.link_external_id(&two.id, &asin).await;

	assert!(
		clash.is_err(),
		"an identifier that resolved to two books would defeat the point of having works at all"
	);
	assert_eq!(repo.find_by_external_id(&asin).await.unwrap(), Some(one.id));
}

#[tokio::test]
async fn external_ids_lists_what_a_work_is_known_by() {
	let db = memory_db().await;
	let repo = repo(&db);
	let hobbit = work("The Hobbit");
	repo.insert(&hobbit).await.unwrap();
	repo.link_external_id(&hobbit.id, &ExternalId::asin("B1"))
		.await
		.unwrap();
	repo.link_external_id(&hobbit.id, &ExternalId::isbn("978"))
		.await
		.unwrap();

	let ids = repo.external_ids(&hobbit.id).await.unwrap();
	assert_eq!(
		ids,
		vec![ExternalId::asin("B1"), ExternalId::isbn("978")],
		"ordering is stable so a caller can rely on it"
	);
	assert!(repo.external_ids("unknown").await.unwrap().is_empty());
}

#[tokio::test]
async fn an_unknown_scheme_in_the_database_is_rejected_not_guessed() {
	let db = memory_db().await;
	let repo = repo(&db);
	let hobbit = work("The Hobbit");
	repo.insert(&hobbit).await.unwrap();

	sqlx::query("INSERT INTO work_external_ids (scheme, value, work_id) VALUES ($1, $2, $3)")
		.bind("olid")
		.bind("OL123M")
		.bind(&hobbit.id)
		.execute(&db)
		.await
		.unwrap();

	let ids = repo.external_ids(&hobbit.id).await;
	assert!(
		matches!(ids, Err(Error::Validation(_))),
		"a scheme this build does not know must surface, not silently vanish from the id set"
	);
	assert_eq!(IdScheme::Asin.as_str(), "asin");
}

#[tokio::test]
async fn an_unidentified_work_matches_regardless_of_case_or_spacing() {
	let db = memory_db().await;
	let repo = repo(&db);
	let mut hobbit = work("The Hobbit");
	hobbit.author = Some("J.R.R. Tolkien".to_string());
	repo.insert(&hobbit).await.unwrap();

	for (title, author) in [
		("The Hobbit", "J.R.R. Tolkien"),
		("  the   hobbit ", "j.r.r. tolkien"),
		("THE HOBBIT", "J.R.R. TOLKIEN"),
	] {
		assert_eq!(
			repo.find_unidentified_by_title(title, Some(author))
				.await
				.unwrap()
				.as_deref(),
			Some(hobbit.id.as_str()),
			"{title:?} by {author:?} is the same ask"
		);
	}
}

#[tokio::test]
async fn a_work_with_an_external_id_is_never_matched_by_title() {
	let db = memory_db().await;
	let repo = repo(&db);
	let hobbit = work("The Hobbit");
	repo.insert(&hobbit).await.unwrap();
	repo.link_external_id(&hobbit.id, &ExternalId::asin("B1"))
		.await
		.unwrap();

	assert!(
		repo.find_unidentified_by_title("The Hobbit", Some("J.R.R. Tolkien"))
			.await
			.unwrap()
			.is_none(),
		"a hand-typed title must not attach itself to a metadata-backed book"
	);
}

#[tokio::test]
async fn linking_an_id_already_claimed_by_another_work_is_a_no_op() {
	let db = memory_db().await;
	let repo = repo(&db);
	let one = work("One");
	let two = work("Two");
	repo.insert(&one).await.unwrap();
	repo.insert(&two).await.unwrap();

	let isbn = ExternalId::isbn("9780007487295");
	repo.link_external_id_if_absent(&one.id, &isbn)
		.await
		.unwrap();
	repo.link_external_id_if_absent(&two.id, &isbn)
		.await
		.unwrap();

	assert_eq!(
		repo.find_by_external_id(&isbn).await.unwrap(),
		Some(one.id),
		"the first claim stands and the second is silently ignored, not an error"
	);
}

#[tokio::test]
async fn a_backfilled_work_is_normalized_by_rust_not_by_sql() {
	let db = memory_db().await;

	sqlx::query(
		"INSERT INTO works (id, title, author, cover_url, created_at) VALUES ($1, $2, $3, NULL, $4)",
	)
	.bind("legacy")
	.bind("LES MIS\u{c9}RABLES")
	.bind("Victor  Hugo")
	.bind(format_dt(Utc::now()))
	.execute(&db)
	.await
	.unwrap();

	crate::run_migrations(&db).await.unwrap();

	let repo = repo(&db);
	assert_eq!(
		repo.find_unidentified_by_title("les mis\u{e9}rables", Some("victor hugo"))
			.await
			.unwrap()
			.as_deref(),
		Some("legacy"),
		"a row the migration created must match the same policy new rows are written with"
	);
}

mod resolving;
