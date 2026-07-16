use super::super::builders::*;
use super::super::*;

#[tokio::test]
async fn manual_request_creates_no_asin_request_and_grabs() {
	let db = memory_db().await;
	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let requests = request_service(&db, jobs.clone());
	let admin = caller("admin", Role::Admin);

	let request = requests
		.create_manual(
			&admin,
			lunu_core::services::ManualRequest {
				title: "Some Obscure Audiobook".to_string(),
				author: Some("Nobody".to_string()),
				notes: None,
				quality_profile_id: None,
			},
		)
		.await
		.unwrap();

	assert!(request.asin.is_none());
	assert!(
		!request.work_id.is_empty(),
		"a request with no identifier still names a work, which is what makes it dedupable"
	);
	assert_eq!(request.title, "Some Obscure Audiobook");
	assert_eq!(request.status, RequestStatus::Approved);

	let grabs = jobs
		.list()
		.await
		.unwrap()
		.into_iter()
		.filter(|job| job.job_type == JobType::Grab)
		.count();
	assert_eq!(grabs, 1);
}

#[tokio::test]
async fn manual_request_rejects_blank_title() {
	let db = memory_db().await;
	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let requests = request_service(&db, jobs);
	let admin = caller("admin", Role::Admin);

	let result = requests
		.create_manual(
			&admin,
			lunu_core::services::ManualRequest {
				title: "   ".to_string(),
				author: None,
				notes: None,
				quality_profile_id: None,
			},
		)
		.await;
	assert!(matches!(result, Err(Error::Validation(_))));
}

#[tokio::test]
async fn the_same_manual_request_twice_is_a_duplicate() {
	let db = memory_db().await;
	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let requests = request_service(&db, jobs.clone());
	let admin = caller("admin", Role::Admin);

	let ask = |title: &str, author: &str| lunu_core::services::ManualRequest {
		title: title.to_string(),
		author: Some(author.to_string()),
		notes: None,
		quality_profile_id: None,
	};

	let first = requests
		.create_manual(&admin, ask("Some Obscure Audiobook", "Nobody"))
		.await
		.unwrap();
	let again = requests
		.create_manual(&admin, ask("some obscure audiobook", "nobody"))
		.await;

	assert!(
		matches!(again, Err(Error::Conflict(_))),
		"an identifier-less request used to escape dedup entirely because null never equals null"
	);

	let different = requests
		.create_manual(&admin, ask("A Different Book", "Nobody"))
		.await
		.unwrap();
	assert_ne!(
		different.work_id, first.work_id,
		"a different title is a different work"
	);
}

#[tokio::test]
async fn two_users_may_each_request_the_same_unidentified_book() {
	let db = memory_db().await;
	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let requests = request_service(&db, jobs.clone());

	let ask = || lunu_core::services::ManualRequest {
		title: "Shared Obscure Book".to_string(),
		author: None,
		notes: None,
		quality_profile_id: None,
	};

	let one = requests
		.create_manual(&caller("admin", Role::Admin), ask())
		.await
		.unwrap();
	let two = requests
		.create_manual(&caller("admin2", Role::Admin), ask())
		.await
		.unwrap();

	assert_eq!(
		one.work_id, two.work_id,
		"both are asking for the same book, so they share a work"
	);
	assert_ne!(one.id, two.id, "but they are two separate requests");
}
