use lunu_core::models::{ExternalId, RequestStatus, Role};

use super::builders::*;
use super::*;

#[tokio::test]
async fn a_watch_is_listed_and_scoped_to_its_owner() {
	let db = memory_db().await;
	let jobs = std::sync::Arc::new(JobService::new(std::sync::Arc::new(SqlxJobRepo::new(
		db.clone(),
	))));
	let watches = watch_service(&db, jobs);
	let owner = caller("u1", Role::User);
	let other = caller("u2", Role::User);

	let watch = watches
		.create(&owner, &ExternalId::asin("B1"))
		.await
		.unwrap();
	assert_eq!(watch.title, "The Hobbit");
	assert_eq!(watch.asin.as_deref(), Some("B1"));

	assert_eq!(watches.count("u1").await.unwrap(), 1);
	assert_eq!(
		watches.count("u2").await.unwrap(),
		0,
		"a watch belongs only to the user who added it"
	);
	assert!(
		watches.delete(&other, &watch.id).await.is_err(),
		"another user must not be able to remove someone's watch"
	);
}

#[tokio::test]
async fn the_same_book_cannot_be_watched_twice() {
	let db = memory_db().await;
	let jobs = std::sync::Arc::new(JobService::new(std::sync::Arc::new(SqlxJobRepo::new(
		db.clone(),
	))));
	let watches = watch_service(&db, jobs);
	let owner = caller("u1", Role::User);

	watches
		.create(&owner, &ExternalId::asin("B1"))
		.await
		.unwrap();
	assert!(
		watches
			.create(&owner, &ExternalId::asin("B1"))
			.await
			.is_err(),
		"the unique index must reject a duplicate watch rather than pile up rows"
	);
}

#[tokio::test]
async fn promoting_a_watch_creates_a_request_and_clears_the_watch() {
	let db = memory_db().await;
	let jobs = std::sync::Arc::new(JobService::new(std::sync::Arc::new(SqlxJobRepo::new(
		db.clone(),
	))));
	let watches = watch_service(&db, jobs);
	let owner = caller("u1", Role::User);

	let watch = watches
		.create(&owner, &ExternalId::asin("B1"))
		.await
		.unwrap();
	let request = watches.promote(&owner, &watch.id).await.unwrap();

	assert_eq!(request.user_id, "u1");
	assert_eq!(request.asin.as_deref(), Some("B1"));
	assert_eq!(
		request.status,
		RequestStatus::Pending,
		"a promoted watch enters the normal approval flow, not straight to grab"
	);
	assert_eq!(
		watches.count("u1").await.unwrap(),
		0,
		"once requested a book leaves the watchlist"
	);
	assert!(
		watches.delete(&owner, &watch.id).await.is_err(),
		"the promoted watch is gone"
	);
}
