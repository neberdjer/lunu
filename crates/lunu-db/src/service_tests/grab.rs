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

struct OkClient;

#[async_trait]
impl DownloadClient for OkClient {
	fn id(&self) -> &'static str {
		"ok"
	}
	async fn add(&self, _url: &str, _category: &str) -> CoreResult<()> {
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
		Arc::new(OkClient),
		jobs.clone(),
	);

	let selection = lunu_core::services::ReleaseSelection {
		title: "The Hobbit [M4B]".to_string(),
		indexer: "MAM".to_string(),
		download_url: "https://tracker/file.torrent".to_string(),
		info_hash: None,
	};
	grabs.grab("r1", Some(selection)).await.unwrap();

	let request = SqlxRequestRepo::new(db.clone())
		.find_by_id("r1")
		.await
		.unwrap()
		.unwrap();
	assert_eq!(request.status, RequestStatus::Failed);
	let monitors = jobs
		.list()
		.await
		.unwrap()
		.into_iter()
		.filter(|job| job.job_type == JobType::MonitorDownload)
		.count();
	assert_eq!(monitors, 0);
}
