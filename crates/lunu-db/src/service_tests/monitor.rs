use lunu_core::services::ClientRoster;

use super::builders::*;
use super::*;

#[derive(Default)]
pub(super) struct FakeClient {
	response: Option<DownloadStatus>,
	removals: std::sync::Mutex<Vec<(String, bool)>>,
}

impl FakeClient {
	pub(super) fn responding(response: Option<DownloadStatus>) -> Self {
		Self {
			response,
			removals: std::sync::Mutex::new(Vec::new()),
		}
	}

	pub(super) fn removals(&self) -> Vec<(String, bool)> {
		self.removals.lock().unwrap().clone()
	}
}

#[async_trait]
impl DownloadClient for FakeClient {
	fn id(&self) -> &'static str {
		"qbittorrent"
	}
	fn protocol(&self) -> Protocol {
		Protocol::Torrent
	}
	async fn is_configured(&self) -> CoreResult<bool> {
		Ok(true)
	}
	async fn add(&self, _download_url: &str, _category: &str) -> CoreResult<Option<String>> {
		Ok(None)
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
		ClientRoster::new(vec![client]),
		request_service(&db, jobs.clone()),
		jobs.clone(),
		Arc::new(NoopPublisher),
		settings_service(&db),
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
		ClientRoster::new(vec![client]),
		request_service(&db, jobs.clone()),
		jobs.clone(),
		Arc::new(NoopPublisher),
		settings_service(&db),
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
		ClientRoster::new(vec![client.clone()]),
		request_service(&db, jobs.clone()),
		jobs.clone(),
		Arc::new(NoopPublisher),
		settings_service(&db),
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

pub(super) fn monitor_with(
	db: &Db,
	jobs: Arc<JobService>,
	client: Arc<FakeClient>,
) -> MonitorService {
	MonitorService::new(
		Arc::new(SqlxDownloadRepo::new(db.clone())),
		ClientRoster::new(vec![client]),
		request_service(db, jobs.clone()),
		jobs,
		Arc::new(NoopPublisher),
		settings_service(db),
	)
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
		ClientRoster::new(vec![client.clone()]),
		request_service(&db, jobs.clone()),
		jobs.clone(),
		Arc::new(NoopPublisher),
		settings_service(&db),
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
