use super::builders::*;
use super::*;

#[tokio::test]
async fn cannot_disable_or_delete_last_admin() {
	let db = memory_db().await;
	let admin = auth_service(&db)
		.setup_first_admin("admin", "password123", None)
		.await
		.unwrap()
		.user;
	let users = user_service(&db);

	assert!(matches!(
		users.set_enabled(&admin.id, false).await,
		Err(Error::Conflict(_))
	));
	assert!(matches!(
		users.delete(&admin.id).await,
		Err(Error::Conflict(_))
	));

	let second = users
		.create("admin2", "password123", None, Role::Admin)
		.await
		.unwrap();
	users.delete(&admin.id).await.unwrap();
	assert!(matches!(
		users.delete(&second.id).await,
		Err(Error::Conflict(_))
	));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_admin_removal_never_drops_below_one() {
	let db = memory_db().await;
	let first = auth_service(&db)
		.setup_first_admin("admin1", "password123", None)
		.await
		.unwrap()
		.user;
	let users = Arc::new(user_service(&db));
	let second = users
		.create("admin2", "password123", None, Role::Admin)
		.await
		.unwrap();

	let a = users.clone();
	let b = users.clone();
	let first_id = first.id.clone();
	let second_id = second.id.clone();
	let t1 = tokio::spawn(async move { a.set_enabled(&first_id, false).await });
	let t2 = tokio::spawn(async move { b.delete(&second_id).await.map(|_| ()) });
	let r1 = t1.await.unwrap();
	let r2 = t2.await.unwrap();

	let failures = [r1.is_err(), r2.is_err()]
		.into_iter()
		.filter(|e| *e)
		.count();
	assert_eq!(
		failures, 1,
		"exactly one concurrent admin removal must be rejected"
	);
	assert!(
		SqlxUserRepo::new(db.clone())
			.count_enabled_admins_excluding("none")
			.await
			.unwrap() >= 1
	);
}

#[tokio::test]
async fn create_initial_admin_rejects_second_insert() {
	let db = memory_db().await;
	let repo = SqlxUserRepo::new(db.clone());
	let mk = |id: &str, name: &str| User {
		id: id.to_string(),
		username: name.to_string(),
		email: None,
		password_hash: Some("h".to_string()),
		role: Role::Admin,
		auth_source: AuthSource::Local,
		display_name: None,
		locale: None,
		enabled: true,
		email_verified: true,
		created_at: Utc::now(),
		updated_at: Utc::now(),
	};

	assert!(repo.create_initial_admin(&mk("1", "a")).await.unwrap());
	assert!(!repo.create_initial_admin(&mk("2", "b")).await.unwrap());
	assert_eq!(repo.count().await.unwrap(), 1);
}

#[tokio::test]
async fn status_by_asin_scopes_to_user_and_keeps_newest() {
	let db = memory_db().await;
	let repo = SqlxRequestRepo::new(db.clone());

	let make =
		|id: &str, user: &str, asin: &str, status: RequestStatus, at: chrono::DateTime<Utc>| {
			Request {
				id: id.to_string(),
				user_id: user.to_string(),
				asin: Some(asin.to_string()),
				title: "t".to_string(),
				author: None,
				cover_url: None,
				status,
				approved_by: None,
				notes: None,
				quality_profile_id: None,
				created_at: at,
				updated_at: at,
			}
		};
	let older = Utc::now() - chrono::Duration::days(1);
	let newer = Utc::now();
	repo.create(&make("1", "u1", "asinX", RequestStatus::Declined, older))
		.await
		.unwrap();
	repo.create(&make("2", "u1", "asinX", RequestStatus::Available, newer))
		.await
		.unwrap();
	repo.create(&make("3", "u1", "asinY", RequestStatus::Pending, newer))
		.await
		.unwrap();
	repo.create(&make("4", "u2", "asinZ", RequestStatus::Pending, newer))
		.await
		.unwrap();

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let service = request_service(&db, jobs);

	let page = [
		"asinX".to_string(),
		"asinY".to_string(),
		"asinZ".to_string(),
	];
	let map = service.status_by_asin("u1", &page).await.unwrap();
	assert_eq!(map.len(), 2);
	assert_eq!(map.get("asinX"), Some(&RequestStatus::Available));
	assert_eq!(map.get("asinY"), Some(&RequestStatus::Pending));
	assert!(
		!map.contains_key("asinZ"),
		"another user's request must not leak into this user's statuses"
	);

	assert!(
		service.status_by_asin("u1", &[]).await.unwrap().is_empty(),
		"an empty page must not query at all"
	);
	assert!(
		!service
			.status_by_asin("u1", &["asinY".to_string()])
			.await
			.unwrap()
			.contains_key("asinX"),
		"only the requested page's asins are returned"
	);

	assert_eq!(
		service.status_for_asin("u1", "asinX").await.unwrap(),
		Some(RequestStatus::Available)
	);
	assert_eq!(
		service.status_for_asin("u1", "missing").await.unwrap(),
		None
	);
}
