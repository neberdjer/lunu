use lunu_core::services::{GrabService, ReleaseSelection};

use super::builders::*;
use super::*;

struct NoopIndexer;

#[async_trait]
impl Indexer for NoopIndexer {
	fn id(&self) -> &'static str {
		"noop"
	}
	async fn search(&self, _query: &str) -> CoreResult<Vec<Release>> {
		Ok(Vec::new())
	}
	async fn test_connection(&self) -> CoreResult<()> {
		Ok(())
	}
}

#[derive(Default)]
struct OkClient {
	adds: std::sync::Mutex<Vec<String>>,
}

impl OkClient {
	fn adds(&self) -> Vec<String> {
		self.adds.lock().unwrap().clone()
	}
}

#[async_trait]
impl DownloadClient for OkClient {
	fn id(&self) -> &'static str {
		"ok"
	}
	async fn add(&self, url: &str, _category: &str) -> CoreResult<()> {
		self.adds.lock().unwrap().push(url.to_string());
		Ok(())
	}
	async fn status(&self, _info_hash: &str) -> CoreResult<Option<DownloadStatus>> {
		Ok(None)
	}
	async fn remove(&self, _info_hash: &str, _delete_files: bool) -> CoreResult<()> {
		Ok(())
	}
	async fn test_connection(&self) -> CoreResult<()> {
		Ok(())
	}
}

fn grab_service(db: &Db, jobs: Arc<JobService>, client: Arc<OkClient>) -> GrabService {
	let releases = Arc::new(ReleaseService::new(
		Arc::new(NoopIndexer),
		Arc::new(SqlxQualityProfileRepo::new(db.clone())),
		Arc::new(SqlxRequestRepo::new(db.clone())),
		Arc::new(SqlxBlocklistRepo::new(db.clone())),
	));
	GrabService::new(
		Arc::new(SqlxDownloadRepo::new(db.clone())),
		request_service(db, jobs.clone()),
		releases,
		client,
		jobs,
	)
}

fn selection(info_hash: Option<&str>) -> ReleaseSelection {
	ReleaseSelection {
		title: "The Hobbit [M4B]".to_string(),
		indexer: "MAM".to_string(),
		download_url: "https://tracker/file.torrent".to_string(),
		info_hash: info_hash.map(str::to_string),
	}
}

async fn monitor_jobs(jobs: &JobService) -> Vec<Job> {
	jobs.list()
		.await
		.unwrap()
		.into_iter()
		.filter(|job| job.job_type == JobType::MonitorDownload)
		.collect()
}

#[tokio::test]
async fn grab_without_info_hash_fails_request_instead_of_stranding() {
	let db = memory_db().await;
	let now = Utc::now();
	SqlxRequestRepo::new(db.clone())
		.create(&Request {
			id: "r1".to_string(),
			user_id: "u1".to_string(),
			asin: Some("B01".to_string()),
			title: "The Hobbit".to_string(),
			author: None,
			cover_url: None,
			status: RequestStatus::Approved,
			approved_by: Some("admin".to_string()),
			notes: None,
			quality_profile_id: None,
			created_at: now,
			updated_at: now,
		})
		.await
		.unwrap();

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let releases = Arc::new(ReleaseService::new(
		Arc::new(NoopIndexer),
		Arc::new(SqlxQualityProfileRepo::new(db.clone())),
		Arc::new(SqlxRequestRepo::new(db.clone())),
		Arc::new(SqlxBlocklistRepo::new(db.clone())),
	));
	let grabs = lunu_core::services::GrabService::new(
		Arc::new(SqlxDownloadRepo::new(db.clone())),
		request_service(&db, jobs.clone()),
		releases,
		Arc::new(OkClient::default()),
		jobs.clone(),
	);

	grabs.grab("r1", Some(selection(None))).await.unwrap();

	let request = SqlxRequestRepo::new(db.clone())
		.find_by_id("r1")
		.await
		.unwrap()
		.unwrap();
	assert_eq!(request.status, RequestStatus::Failed);
	assert_eq!(monitor_jobs(&jobs).await.len(), 0);

	let download = SqlxDownloadRepo::new(db.clone())
		.find_by_request("r1")
		.await
		.unwrap()
		.unwrap();
	assert_eq!(
		download.state,
		DownloadState::Failed,
		"an untrackable download must be Failed so retry is not blocked by the active-download guard"
	);
}

#[tokio::test]
async fn grab_resumes_when_the_monitor_job_was_never_enqueued() {
	let db = memory_db().await;
	seed_download(&db, Utc::now()).await;
	let downloads = SqlxDownloadRepo::new(db.clone());
	downloads
		.update_status("d1", DownloadState::Queued, 0, Utc::now())
		.await
		.unwrap();

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let client = Arc::new(OkClient::default());
	let grabs = grab_service(&db, jobs.clone(), client.clone());

	assert_eq!(
		monitor_jobs(&jobs).await.len(),
		0,
		"precondition: the earlier attempt died before enqueueing a monitor"
	);

	grabs.grab("r1", None).await.unwrap();

	let monitors = monitor_jobs(&jobs).await;
	assert_eq!(
		monitors.len(),
		1,
		"resume must enqueue the monitor the crashed attempt never did"
	);
	let payload: MonitorPayload = ::serde_json::from_str(&monitors[0].payload).unwrap();
	assert_eq!(
		payload.download_id, "d1",
		"resume must monitor the existing download, not a new one"
	);
	assert!(
		client.adds().is_empty(),
		"resume must not re-add the torrent to the client"
	);
	assert_eq!(
		downloads.list_page(50, 0).await.unwrap().len(),
		1,
		"resume must not create a duplicate download row"
	);

	grabs.grab("r1", None).await.unwrap();
	assert_eq!(
		monitor_jobs(&jobs).await.len(),
		1,
		"a second resume must not fork a duplicate monitor chain"
	);
}

#[tokio::test]
async fn grab_refuses_a_manual_selection_while_a_download_is_active() {
	let db = memory_db().await;
	seed_download(&db, Utc::now()).await;

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let client = Arc::new(OkClient::default());
	let grabs = grab_service(&db, jobs.clone(), client.clone());

	let result = grabs.grab("r1", Some(selection(Some("deadbeef")))).await;

	assert!(
		matches!(result, Err(Error::Conflict(_))),
		"an explicit release override must not be silently ignored"
	);
	assert!(
		client.adds().is_empty(),
		"the refused grab must not touch the client"
	);
}

#[tokio::test]
async fn grab_allows_a_new_release_after_the_previous_download_failed() {
	let db = memory_db().await;
	seed_download(&db, Utc::now()).await;
	SqlxDownloadRepo::new(db.clone())
		.update_status("d1", DownloadState::Failed, 0, Utc::now())
		.await
		.unwrap();

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let client = Arc::new(OkClient::default());
	let grabs = grab_service(&db, jobs.clone(), client.clone());

	grabs
		.grab("r1", Some(selection(Some("deadbeef"))))
		.await
		.unwrap();

	assert_eq!(
		client.adds(),
		vec!["https://tracker/file.torrent".to_string()],
		"a failed download must not block re-grabbing a different release"
	);
}
