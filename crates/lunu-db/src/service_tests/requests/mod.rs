use super::builders::*;
use super::*;

#[tokio::test]
async fn request_list_page_filters_and_counts() {
	let db = memory_db().await;
	let repo = SqlxRequestRepo::new(db.clone());
	let now = Utc::now();

	let make = |id: &str, user: &str, status: RequestStatus| Request {
		user_id: user.to_string(),
		status,
		created_at: now,
		updated_at: now,
		..request(id)
	};
	repo.create(&make("a", "u1", RequestStatus::Pending))
		.await
		.unwrap();
	repo.create(&make("b", "u1", RequestStatus::Approved))
		.await
		.unwrap();
	repo.create(&make("c", "u2", RequestStatus::Pending))
		.await
		.unwrap();

	assert_eq!(repo.count(None, None).await.unwrap(), 3);
	assert_eq!(repo.count(None, Some("pending")).await.unwrap(), 2);
	assert_eq!(repo.count(Some("u1"), None).await.unwrap(), 2);
	assert_eq!(repo.count(Some("u1"), Some("pending")).await.unwrap(), 1);

	assert_eq!(repo.list_page(None, None, 2, 0).await.unwrap().len(), 2);
	assert_eq!(repo.list_page(None, None, 2, 2).await.unwrap().len(), 1);

	let pending = repo.list_page(None, Some("pending"), 10, 0).await.unwrap();
	assert_eq!(pending.len(), 2);
	assert!(pending.iter().all(|r| r.status == RequestStatus::Pending));
}

struct FakeIndexer {
	releases: Vec<Release>,
}

#[async_trait]
impl Indexer for FakeIndexer {
	fn id(&self) -> &'static str {
		"fake"
	}
	async fn search(&self, _query: &str) -> CoreResult<Vec<Release>> {
		Ok(self.releases.clone())
	}
	async fn test_connection(&self) -> CoreResult<()> {
		Ok(())
	}
}

#[tokio::test]
async fn delete_request_cascades_to_downloads_and_activity() {
	let db = memory_db().await;
	seed_download(&db, Utc::now()).await;
	SqlxActivityRepo::new(db.clone())
		.create(&Activity {
			id: "act1".to_string(),
			request_id: Some("r1".to_string()),
			media_id: None,
			event: "downloading".to_string(),
			detail: None,
			actor: None,
			at: Utc::now(),
		})
		.await
		.unwrap();

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let requests = request_service(&db, jobs);
	requests
		.delete(&caller("admin", Role::Admin), "r1")
		.await
		.unwrap();

	assert!(
		SqlxRequestRepo::new(db.clone())
			.find_by_id("r1")
			.await
			.unwrap()
			.is_none()
	);
	assert!(
		SqlxDownloadRepo::new(db.clone())
			.find_by_id("d1")
			.await
			.unwrap()
			.is_none()
	);
	assert_eq!(
		SqlxActivityRepo::new(db.clone())
			.for_request("r1")
			.await
			.unwrap()
			.len(),
		0
	);
}

#[tokio::test]
async fn retry_reopens_failed_request_and_enqueues_grab() {
	let db = memory_db().await;
	let now = Utc::now();
	SqlxRequestRepo::new(db.clone())
		.create(&Request {
			work_id: "work-B01".to_string(),
			asin: Some("B01".to_string()),
			title: "Book".to_string(),
			status: RequestStatus::Failed,
			created_at: now,
			updated_at: now,
			..request("r1")
		})
		.await
		.unwrap();

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let requests = request_service(&db, jobs.clone());
	let owner = caller("u1", Role::User);

	let updated = requests.retry(&owner, "r1").await.unwrap();
	assert_eq!(updated.status, RequestStatus::Approved);

	let listed = jobs.list().await.unwrap();
	let grabs: Vec<_> = listed
		.iter()
		.filter(|job| job.job_type == JobType::Grab)
		.collect();
	assert_eq!(grabs.len(), 1);
	assert!(listed.iter().any(|job| job.job_type == JobType::Notify));

	assert!(requests.retry(&owner, "r1").await.is_err());
}

#[tokio::test]
async fn blocklisted_release_excluded_from_for_request() {
	let db = memory_db().await;
	let now = Utc::now();
	SqlxRequestRepo::new(db.clone())
		.create(&Request {
			work_id: "work-B01".to_string(),
			asin: Some("B01".to_string()),
			title: "Book".to_string(),
			created_at: now,
			updated_at: now,
			..request("r1")
		})
		.await
		.unwrap();

	let indexer = Arc::new(FakeIndexer {
		releases: vec![release("magnet:a"), release("magnet:b")],
	});
	let releases = ReleaseService::new(
		indexer,
		Arc::new(SqlxQualityProfileRepo::new(db.clone())),
		Arc::new(SqlxRequestRepo::new(db.clone())),
		Arc::new(SqlxBlocklistRepo::new(db.clone())),
	);

	assert_eq!(releases.for_request("r1").await.unwrap().len(), 2);

	releases.blocklist_release("r1", "magnet:a").await.unwrap();
	let after = releases.for_request("r1").await.unwrap();
	assert_eq!(after.len(), 1);
	assert_eq!(after[0].release.download_url, "magnet:b");
}

#[tokio::test]
async fn download_create_and_set_state() {
	let db = memory_db().await;
	let repo = SqlxDownloadRepo::new(db.clone());

	let now = Utc::now();
	let download = Download {
		id: "d1".to_string(),
		request_id: "r1".to_string(),
		client: "qbittorrent".to_string(),
		category: "lunu".to_string(),
		release_title: "The Hobbit [M4B]".to_string(),
		indexer: "MyTracker".to_string(),
		download_url: "magnet:?xt=urn:btih:abc".to_string(),
		client_ref: Some("abc".to_string()),
		state: DownloadState::Queued,
		progress: 0,
		created_at: now,
		updated_at: now,
	};
	repo.create(&download).await.unwrap();

	let found = repo.find_by_request("r1").await.unwrap().unwrap();
	assert_eq!(found.id, "d1");
	assert_eq!(found.state, DownloadState::Queued);
	assert_eq!(found.release_title, "The Hobbit [M4B]");
	assert_eq!(found.client_ref.as_deref(), Some("abc"));

	repo.update_status("d1", DownloadState::Downloading, 42, Utc::now())
		.await
		.unwrap();
	let updated = repo.find_by_id("d1").await.unwrap().unwrap();
	assert_eq!(updated.state, DownloadState::Downloading);
	assert_eq!(updated.progress, 42);
	assert_eq!(repo.list().await.unwrap().len(), 1);
}

mod manual;
