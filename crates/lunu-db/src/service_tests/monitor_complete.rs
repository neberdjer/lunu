use lunu_core::models::ImportPayload;

use super::builders::*;
use super::monitor::{FakeClient, monitor_with};
use super::*;

async fn import_jobs(jobs: &JobService) -> Vec<Job> {
	jobs.list()
		.await
		.unwrap()
		.into_iter()
		.filter(|job| job.job_type == JobType::Import)
		.collect()
}

fn completed_client(content_path: &str) -> Arc<FakeClient> {
	Arc::new(FakeClient::responding(Some(DownloadStatus {
		state: DownloadState::Completed,
		progress: 1.0,
		content_path: Some(content_path.to_string()),
	})))
}

#[tokio::test]
async fn complete_reenqueues_import_when_marked_importing_but_no_import_job_exists() {
	let db = memory_db().await;
	seed_download(&db, Utc::now()).await;

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	request_service(&db, jobs.clone())
		.mark_importing("r1")
		.await
		.unwrap();
	assert_eq!(
		import_jobs(&jobs).await.len(),
		0,
		"precondition: status committed but the Import enqueue never happened"
	);

	let monitor = monitor_with(&db, jobs.clone(), completed_client("/downloads/The Hobbit"));
	monitor
		.poll(&MonitorPayload {
			download_id: "d1".to_string(),
			misses: 0,
			stalls: 0,
		})
		.await
		.unwrap();

	let imports = import_jobs(&jobs).await;
	assert_eq!(
		imports.len(),
		1,
		"a request stuck at Importing with no Import job must recover, not strand"
	);
	let payload: ImportPayload = serde_json::from_str(&imports[0].payload).unwrap();
	assert_eq!(payload.download_id, "d1");
	assert_eq!(payload.content_path, "/downloads/The Hobbit");
}

#[tokio::test]
async fn complete_does_not_enqueue_a_second_import_when_one_is_active() {
	let db = memory_db().await;
	seed_download(&db, Utc::now()).await;

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let monitor = monitor_with(&db, jobs.clone(), completed_client("/downloads/The Hobbit"));
	let payload = MonitorPayload {
		download_id: "d1".to_string(),
		misses: 0,
		stalls: 0,
	};

	monitor.poll(&payload).await.unwrap();
	monitor.poll(&payload).await.unwrap();

	assert_eq!(
		import_jobs(&jobs).await.len(),
		1,
		"a re-run must not duplicate the import"
	);
}

#[tokio::test]
async fn complete_does_nothing_once_the_request_is_available() {
	let db = memory_db().await;
	seed_download(&db, Utc::now()).await;

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	request_service(&db, jobs.clone())
		.mark_available("r1")
		.await
		.unwrap();

	let monitor = monitor_with(&db, jobs.clone(), completed_client("/downloads/The Hobbit"));
	monitor
		.poll(&MonitorPayload {
			download_id: "d1".to_string(),
			misses: 0,
			stalls: 0,
		})
		.await
		.unwrap();

	assert_eq!(
		import_jobs(&jobs).await.len(),
		0,
		"an already-imported request must not be re-imported"
	);
}

#[tokio::test]
async fn complete_fails_the_download_when_the_content_path_is_unsafe() {
	let db = memory_db().await;
	seed_download(&db, Utc::now()).await;

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let downloads = Arc::new(SqlxDownloadRepo::new(db.clone()));
	let monitor = monitor_with(&db, jobs.clone(), completed_client("/downloads/../../etc"));
	monitor
		.poll(&MonitorPayload {
			download_id: "d1".to_string(),
			misses: 0,
			stalls: 0,
		})
		.await
		.unwrap();

	assert_eq!(
		import_jobs(&jobs).await.len(),
		0,
		"a traversing content path must never reach the importer"
	);
	assert_eq!(
		downloads.find_by_id("d1").await.unwrap().unwrap().state,
		DownloadState::Failed
	);
}
