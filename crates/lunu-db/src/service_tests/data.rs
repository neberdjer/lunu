use super::builders::*;
use super::*;

#[tokio::test]
async fn create_within_quota_enforces_limit() {
	let db = memory_db().await;
	let repo = SqlxRequestRepo::new(db.clone());
	let since = Utc::now() - chrono::Duration::days(30);
	let mk = |id: &str, asin: &str| Request {
		work_id: format!("work-{asin}"),
		format: Format::Audiobook,
		id: id.to_string(),
		user_id: "u1".to_string(),
		asin: Some(asin.to_string()),
		title: "t".to_string(),
		author: None,
		cover_url: None,
		series_name: None,
		series_sequence: None,
		status: RequestStatus::Pending,
		approved_by: None,
		notes: None,
		quality_profile_id: None,
		created_at: Utc::now(),
		updated_at: Utc::now(),
	};

	assert!(
		repo.create_within_quota(&mk("1", "x"), 2, since)
			.await
			.unwrap()
	);
	assert!(
		repo.create_within_quota(&mk("2", "y"), 2, since)
			.await
			.unwrap()
	);
	assert!(
		!repo
			.create_within_quota(&mk("3", "z"), 2, since)
			.await
			.unwrap()
	);
	assert_eq!(repo.count(Some("u1"), None).await.unwrap(), 2);
}

#[tokio::test]
async fn metadata_cache_put_get_upsert() {
	let db = memory_db().await;
	let cache = SqlxMetadataCacheRepo::new(db.clone());

	assert!(
		cache
			.get("audnexus", "book", "B01")
			.await
			.unwrap()
			.is_none()
	);

	let entry = |payload: &str| MetadataCacheEntry {
		provider: "audnexus".to_string(),
		kind: "book".to_string(),
		key: "B01".to_string(),
		payload: payload.to_string(),
		fetched_at: Utc::now(),
	};

	cache.put(&entry("{\"v\":1}")).await.unwrap();
	assert_eq!(
		cache
			.get("audnexus", "book", "B01")
			.await
			.unwrap()
			.unwrap()
			.payload,
		"{\"v\":1}"
	);

	cache.put(&entry("{\"v\":2}")).await.unwrap();
	assert_eq!(
		cache
			.get("audnexus", "book", "B01")
			.await
			.unwrap()
			.unwrap()
			.payload,
		"{\"v\":2}"
	);
}

#[tokio::test]
async fn request_lifecycle_and_quota_count() {
	let db = memory_db().await;
	let requests = SqlxRequestRepo::new(db.clone());

	let now = Utc::now();
	let request = Request {
		work_id: "work-B01".to_string(),
		asin: Some("B01".to_string()),
		title: "The Hobbit".to_string(),
		author: Some("Tolkien".to_string()),
		created_at: now,
		updated_at: now,
		..request("r1")
	};
	requests.create(&request).await.unwrap();

	let found = requests
		.find_by_user_and_work("u1", "work-B01")
		.await
		.unwrap()
		.unwrap();
	assert_eq!(found.status, RequestStatus::Pending);
	assert_eq!(found.author.as_deref(), Some("Tolkien"));

	let mut approved = found;
	approved.status = RequestStatus::Approved;
	approved.approved_by = Some("admin".to_string());
	requests.update(&approved).await.unwrap();
	assert_eq!(
		requests.find_by_id("r1").await.unwrap().unwrap().status,
		RequestStatus::Approved
	);

	assert_eq!(requests.list_for_user("u1").await.unwrap().len(), 1);
}

#[tokio::test]
async fn quality_profile_crud_and_default() {
	let db = memory_db().await;
	let repo = SqlxQualityProfileRepo::new(db.clone());

	let now = Utc::now();
	let profile = QualityProfile {
		id: "p1".to_string(),
		name: "Audiobook".to_string(),
		allowed_formats: vec!["m4b".to_string(), "mp3".to_string()],
		preferred_formats: vec!["m4b".to_string()],
		min_seeders: 2,
		min_size_mb: Some(10),
		max_size_mb: None,
		seeder_weight: 1,
		format_weight: 100,
		preferred_keywords: vec!["unabridged".to_string()],
		avoided_keywords: vec!["abridged".to_string()],
		keyword_weight: 50,
		preferred_protocol: Some(lunu_core::models::Protocol::Usenet),
		protocol_weight: 75,
		min_bitrate_kbps: Some(64),
		bitrate_weight: 3,
		allowed_languages: vec!["en".to_string(), "de".to_string()],
		freeleech_weight: 250,
		is_default: true,
		created_at: now,
		updated_at: now,
	};
	repo.create(&profile).await.unwrap();

	let loaded = repo.find_by_id("p1").await.unwrap().unwrap();
	assert_eq!(loaded.allowed_formats, vec!["m4b", "mp3"]);
	assert_eq!(loaded.preferred_keywords, vec!["unabridged"]);
	assert_eq!(loaded.avoided_keywords, vec!["abridged"]);
	assert_eq!(loaded.keyword_weight, 50);
	assert_eq!(loaded.min_seeders, 2);
	assert_eq!(loaded.min_size_mb, Some(10));
	assert_eq!(
		loaded.preferred_protocol,
		Some(lunu_core::models::Protocol::Usenet),
		"a stated protocol preference survives the round trip"
	);
	assert_eq!(loaded.protocol_weight, 75);
	assert_eq!(loaded.min_bitrate_kbps, Some(64));
	assert_eq!(loaded.bitrate_weight, 3);
	assert_eq!(loaded.allowed_languages, vec!["en", "de"]);
	assert_eq!(loaded.freeleech_weight, 250);
	assert!(loaded.is_default);

	let mut cleared = profile.clone();
	cleared.preferred_protocol = None;
	repo.update(&cleared).await.unwrap();
	assert_eq!(
		repo.find_by_id("p1")
			.await
			.unwrap()
			.unwrap()
			.preferred_protocol,
		None,
		"clearing the preference must persist as no preference"
	);
	repo.update(&profile).await.unwrap();

	assert_eq!(repo.find_default().await.unwrap().unwrap().id, "p1");

	let mut second = profile.clone();
	second.id = "p2".to_string();
	second.is_default = false;
	repo.create(&second).await.unwrap();

	repo.set_default("p2").await.unwrap();
	assert_eq!(repo.find_default().await.unwrap().unwrap().id, "p2");
	assert!(!repo.find_by_id("p1").await.unwrap().unwrap().is_default);
	assert_eq!(repo.list().await.unwrap().len(), 2);
}
