use chrono::Duration;
use lunu_core::models::{ExternalId, Format, Media, MediaSource, Request, RequestStatus, Work};
use lunu_core::repo::{ActivityRepo, MediaRepo, RequestRepo, WorkRepo};

use super::super::*;
use super::{repo, work};
use crate::repair::merge_duplicate_works;

async fn seed_edition(db: &Db, id: &str, asin: &str, cover: Option<&str>, age_seconds: i64) {
	let repo = repo(db);
	repo.insert(&Work {
		id: id.to_string(),
		cover_url: cover.map(str::to_string),
		created_at: Utc::now() - Duration::seconds(age_seconds),
		..work("The Hobbit")
	})
	.await
	.unwrap();
	repo.link_external_id(id, &ExternalId::asin(asin))
		.await
		.unwrap();
}

async fn seed_request(db: &Db, id: &str, user_id: &str, work_id: &str, status: RequestStatus) {
	SqlxRequestRepo::new(db.clone())
		.create(&Request {
			id: id.to_string(),
			user_id: user_id.to_string(),
			work_id: work_id.to_string(),
			format: Format::Audiobook,
			asin: None,
			title: "The Hobbit".to_string(),
			author: Some("J.R.R. Tolkien".to_string()),
			cover_url: None,
			status,
			approved_by: None,
			notes: None,
			quality_profile_id: None,
			created_at: Utc::now(),
			updated_at: Utc::now(),
		})
		.await
		.unwrap();
}

async fn request_state(db: &Db, id: &str) -> (String, RequestStatus) {
	let request = SqlxRequestRepo::new(db.clone())
		.find_by_id(id)
		.await
		.unwrap()
		.unwrap();
	(request.work_id, request.status)
}

#[tokio::test]
async fn duplicate_editions_collapse_into_the_oldest_work() {
	let db = memory_db().await;
	seed_edition(&db, "w-old", "B-SERKIS", None, 60).await;
	seed_edition(&db, "w-new", "B-INGLIS", Some("cover"), 0).await;

	merge_duplicate_works(&db).await.unwrap();

	let repo = repo(&db);
	for asin in ["B-SERKIS", "B-INGLIS"] {
		assert_eq!(
			repo.find_by_external_id(&ExternalId::asin(asin))
				.await
				.unwrap()
				.as_deref(),
			Some("w-old"),
			"every edition asin must reach the surviving work"
		);
	}
	assert!(repo.find_by_id("w-new").await.unwrap().is_none());
	let survivor = repo.find_by_id("w-old").await.unwrap().unwrap();
	assert_eq!(
		survivor.cover_url.as_deref(),
		Some("cover"),
		"a cover the loser had is better than none"
	);
}

#[tokio::test]
async fn merging_repoints_requests_and_declines_the_colliding_duplicate() {
	let db = memory_db().await;
	seed_edition(&db, "w-old", "B-SERKIS", None, 60).await;
	seed_edition(&db, "w-new", "B-INGLIS", None, 0).await;
	seed_request(&db, "r-kept", "u1", "w-old", RequestStatus::Downloading).await;
	seed_request(&db, "r-dup", "u1", "w-new", RequestStatus::Pending).await;
	seed_request(&db, "r-other", "u2", "w-new", RequestStatus::Pending).await;

	merge_duplicate_works(&db).await.unwrap();

	assert_eq!(
		request_state(&db, "r-kept").await,
		("w-old".to_string(), RequestStatus::Downloading)
	);
	assert_eq!(
		request_state(&db, "r-dup").await,
		("w-old".to_string(), RequestStatus::Declined),
		"a second active request for the same work and format is a duplicate"
	);
	assert_eq!(
		request_state(&db, "r-other").await,
		("w-old".to_string(), RequestStatus::Pending),
		"another user's request moves without losing its place"
	);

	let trail = SqlxActivityRepo::new(db.clone())
		.for_request("r-dup")
		.await
		.unwrap();
	assert_eq!(trail.len(), 1, "a repair decline must leave a trail");
	assert_eq!(trail[0].event, "declined");
	assert!(
		trail[0]
			.detail
			.as_deref()
			.unwrap_or_default()
			.contains("merge")
	);
}

#[tokio::test]
async fn merging_repoints_media() {
	let db = memory_db().await;
	seed_edition(&db, "w-old", "B-SERKIS", None, 60).await;
	seed_edition(&db, "w-new", "B-INGLIS", None, 0).await;
	let media_repo = SqlxMediaRepo::new(db.clone());
	media_repo
		.insert(&Media {
			id: "m1".to_string(),
			work_id: Some("w-new".to_string()),
			format: Format::Audiobook,
			asin: Some("B-INGLIS".to_string()),
			abs_item_id: Some("abs-1".to_string()),
			title: "The Hobbit".to_string(),
			author: Some("J.R.R. Tolkien".to_string()),
			cover_url: None,
			series_name: None,
			series_sequence: None,
			library_path: "/abs/hobbit".to_string(),
			source: MediaSource::Abs,
			overridden: false,
			matched_by: None,
			request_id: None,
			created_at: Utc::now(),
		})
		.await
		.unwrap();

	merge_duplicate_works(&db).await.unwrap();

	let media = media_repo.find_by_id("m1").await.unwrap().unwrap();
	assert_eq!(media.work_id.as_deref(), Some("w-old"));
}

#[tokio::test]
async fn works_without_external_ids_never_merge() {
	let db = memory_db().await;
	let repo = repo(&db);
	for id in ["manual-a", "manual-b"] {
		repo.insert(&Work {
			id: id.to_string(),
			..work("Dune")
		})
		.await
		.unwrap();
	}

	merge_duplicate_works(&db).await.unwrap();

	for id in ["manual-a", "manual-b"] {
		assert!(
			repo.find_by_id(id).await.unwrap().is_some(),
			"hand-typed works carry no ids that prove they are the same book"
		);
	}
}
