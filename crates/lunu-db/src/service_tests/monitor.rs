use super::builders::*;
use super::*;

#[derive(Default)]
struct FakeClient {
	response: Option<DownloadStatus>,
	removals: std::sync::Mutex<Vec<(String, bool)>>,
}

impl FakeClient {
	fn responding(response: Option<DownloadStatus>) -> Self {
		Self {
			response,
			removals: std::sync::Mutex::new(Vec::new()),
		}
	}

	fn removals(&self) -> Vec<(String, bool)> {
		self.removals.lock().unwrap().clone()
	}
}

#[async_trait]
impl DownloadClient for FakeClient {
	fn id(&self) -> &'static str {
		"fake"
	}
	async fn add(&self, _download_url: &str, _category: &str) -> CoreResult<()> {
		Ok(())
	}
	async fn status(&self, _info_hash: &str) -> CoreResult<Option<DownloadStatus>> {
		Ok(self.response.clone())
	}
	async fn remove(&self, info_hash: &str, delete_files: bool) -> CoreResult<()> {
		self.removals
			.lock()
			.unwrap()
			.push((info_hash.to_string(), delete_files));
		Ok(())
	}
	async fn test_connection(&self) -> CoreResult<()> {
		Ok(())
	}
}

#[tokio::test]
async fn monitor_marks_request_importing_on_completion() {
	let db = memory_db().await;
	seed_download(&db, Utc::now()).await;

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let downloads = Arc::new(SqlxDownloadRepo::new(db.clone()));
	let client = Arc::new(FakeClient::responding(Some(DownloadStatus {
		state: DownloadState::Completed,
		progress: 1.0,
		content_path: Some("/library/x".to_string()),
	})));
	let monitor = MonitorService::new(
		downloads.clone(),
		client,
		request_service(&db, jobs.clone()),
		jobs.clone(),
		Arc::new(NoopPublisher),
	);

	monitor
		.poll(&MonitorPayload {
			download_id: "d1".to_string(),
			misses: 0,
			stalls: 0,
		})
		.await
		.unwrap();

	assert_eq!(
		SqlxRequestRepo::new(db.clone())
			.find_by_id("r1")
			.await
			.unwrap()
			.unwrap()
			.status,
		RequestStatus::Importing
	);
	let download = downloads.find_by_id("d1").await.unwrap().unwrap();
	assert_eq!(download.state, DownloadState::Completed);
	assert_eq!(download.progress, 100);
}

#[tokio::test]
async fn monitor_reschedules_while_downloading() {
	let db = memory_db().await;
	seed_download(&db, Utc::now()).await;

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let downloads = Arc::new(SqlxDownloadRepo::new(db.clone()));
	let client = Arc::new(FakeClient::responding(Some(DownloadStatus {
		state: DownloadState::Downloading,
		progress: 0.5,
		content_path: None,
	})));
	let monitor = MonitorService::new(
		downloads.clone(),
		client,
		request_service(&db, jobs.clone()),
		jobs.clone(),
		Arc::new(NoopPublisher),
	);

	monitor
		.poll(&MonitorPayload {
			download_id: "d1".to_string(),
			misses: 0,
			stalls: 0,
		})
		.await
		.unwrap();

	let listed = jobs.list().await.unwrap();
	assert_eq!(listed.len(), 1);
	assert_eq!(listed[0].job_type, JobType::MonitorDownload);
	assert_eq!(
		downloads.find_by_id("d1").await.unwrap().unwrap().progress,
		50
	);
}

#[tokio::test]
async fn monitor_fails_after_max_misses() {
	let db = memory_db().await;
	seed_download(&db, Utc::now()).await;

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let downloads = Arc::new(SqlxDownloadRepo::new(db.clone()));
	let client = Arc::new(FakeClient::responding(None));
	let monitor = MonitorService::new(
		downloads.clone(),
		client.clone(),
		request_service(&db, jobs.clone()),
		jobs.clone(),
		Arc::new(NoopPublisher),
	);

	monitor
		.poll(&MonitorPayload {
			download_id: "d1".to_string(),
			misses: MONITOR_MAX_MISSES - 1,
			stalls: 0,
		})
		.await
		.unwrap();

	assert_eq!(
		downloads.find_by_id("d1").await.unwrap().unwrap().state,
		DownloadState::Failed
	);
	assert_eq!(
		SqlxRequestRepo::new(db.clone())
			.find_by_id("r1")
			.await
			.unwrap()
			.unwrap()
			.status,
		RequestStatus::Failed
	);
	let listed = jobs.list().await.unwrap();
	assert!(
		!listed
			.iter()
			.any(|job| matches!(job.job_type, JobType::Grab | JobType::MonitorDownload))
	);
	assert!(
		client.removals().is_empty(),
		"a torrent missing from the client must never be deleted: it may only be transiently absent"
	);
}

#[tokio::test]
async fn monitor_removes_the_torrent_when_the_client_reports_failure() {
	let db = memory_db().await;
	seed_download(&db, Utc::now()).await;

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let downloads = Arc::new(SqlxDownloadRepo::new(db.clone()));
	let client = Arc::new(FakeClient::responding(Some(DownloadStatus {
		state: DownloadState::Failed,
		progress: 0.25,
		content_path: None,
	})));
	let monitor = MonitorService::new(
		downloads.clone(),
		client.clone(),
		request_service(&db, jobs.clone()),
		jobs.clone(),
		Arc::new(NoopPublisher),
	);

	monitor
		.poll(&MonitorPayload {
			download_id: "d1".to_string(),
			misses: 0,
			stalls: 0,
		})
		.await
		.unwrap();

	assert_eq!(
		downloads.find_by_id("d1").await.unwrap().unwrap().state,
		DownloadState::Failed
	);
	assert_eq!(
		client.removals(),
		vec![("abc".to_string(), true)],
		"a client-confirmed failure removes the torrent and its files"
	);
}

#[tokio::test]
async fn job_claim_is_atomic_and_lifecycle_transitions() {
	let db = memory_db().await;
	let repo = SqlxJobRepo::new(db.clone());

	let now = Utc::now();
	repo.create(&pending_job("j1", now)).await.unwrap();

	let claimed = repo
		.claim_next("worker-a", Utc::now())
		.await
		.unwrap()
		.unwrap();
	assert_eq!(claimed.id, "j1");
	assert_eq!(claimed.status, JobStatus::Running);
	assert_eq!(claimed.attempts, 1);
	assert_eq!(claimed.locked_by.as_deref(), Some("worker-a"));

	assert!(
		repo.claim_next("worker-b", Utc::now())
			.await
			.unwrap()
			.is_none()
	);

	let future = Utc::now() + chrono::Duration::seconds(30);
	repo.reschedule("j1", "worker-a", "temporary", future, Utc::now(), 5)
		.await
		.unwrap();
	let after = repo.find_by_id("j1").await.unwrap().unwrap();
	assert_eq!(after.status, JobStatus::Pending);
	assert_eq!(after.attempts, 1);
	assert_eq!(after.last_error.as_deref(), Some("temporary"));
	assert!(after.locked_by.is_none());

	assert!(
		repo.claim_next("worker-a", Utc::now())
			.await
			.unwrap()
			.is_none()
	);

	let reclaimed = repo
		.claim_next("worker-a", future + chrono::Duration::seconds(1))
		.await
		.unwrap()
		.unwrap();
	assert_eq!(reclaimed.attempts, 2);

	repo.complete("j1", "worker-a", Utc::now()).await.unwrap();
	assert_eq!(
		repo.find_by_id("j1").await.unwrap().unwrap().status,
		JobStatus::Completed
	);
	assert_eq!(repo.list().await.unwrap().len(), 1);
}

#[tokio::test]
async fn reap_stale_returns_running_jobs_to_pending() {
	let db = memory_db().await;
	let repo = SqlxJobRepo::new(db.clone());

	let now = Utc::now();
	repo.create(&pending_job("j2", now)).await.unwrap();
	repo.claim_next("worker-a", now).await.unwrap().unwrap();

	assert_eq!(
		repo.reap_stale(now - chrono::Duration::seconds(300), Utc::now())
			.await
			.unwrap(),
		0
	);

	let reaped = repo
		.reap_stale(now + chrono::Duration::seconds(1), Utc::now())
		.await
		.unwrap();
	assert_eq!(reaped, 1);

	let after = repo.find_by_id("j2").await.unwrap().unwrap();
	assert_eq!(after.status, JobStatus::Pending);
	assert!(after.locked_by.is_none());
	assert_eq!(after.attempts, 1);
}
