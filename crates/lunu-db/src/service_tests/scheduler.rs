use std::sync::Arc;

use lunu_core::consts::jobs::LIBRARY_SYNC_INTERVAL_SECS;
use lunu_core::models::JobType;
use lunu_core::repo::ScheduleRepo;
use lunu_core::services::{JobService, SchedulerService};

use super::*;

fn scheduler(db: &Db) -> (SchedulerService, Arc<JobService>) {
	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let service = SchedulerService::new(Arc::new(SqlxScheduleRepo::new(db.clone())), jobs.clone());
	(service, jobs)
}

async fn library_sync_jobs(jobs: &JobService) -> usize {
	jobs.list()
		.await
		.unwrap()
		.into_iter()
		.filter(|job| job.job_type == JobType::LibrarySync)
		.count()
}

#[tokio::test]
async fn ensure_defaults_seeds_library_sync_disabled() {
	let db = memory_db().await;
	let (service, _) = scheduler(&db);

	service.ensure_defaults().await.unwrap();

	let schedules = service.list().await.unwrap();
	let library = schedules.iter().find(|s| s.kind == "library-sync").unwrap();
	assert!(!library.enabled);
	assert_eq!(library.interval_secs, LIBRARY_SYNC_INTERVAL_SECS);

	assert_eq!(service.run_due().await.unwrap(), 0);
}

#[tokio::test]
async fn enabled_due_schedule_enqueues_and_advances() {
	let db = memory_db().await;
	let (service, jobs) = scheduler(&db);
	service.ensure_defaults().await.unwrap();

	service.configure("library-sync", true, 3600).await.unwrap();
	SqlxScheduleRepo::new(db.clone())
		.advance("library-sync", chrono::Utc::now(), chrono::Utc::now())
		.await
		.unwrap();

	assert_eq!(service.run_due().await.unwrap(), 1);
	assert_eq!(library_sync_jobs(&jobs).await, 1);

	let library = service
		.list()
		.await
		.unwrap()
		.into_iter()
		.find(|s| s.kind == "library-sync")
		.unwrap();
	assert!(library.next_run_at > chrono::Utc::now());
	assert!(library.last_run_at.is_some());
}

#[tokio::test]
async fn dedup_skips_when_active_job_exists() {
	let db = memory_db().await;
	let (service, jobs) = scheduler(&db);
	service.ensure_defaults().await.unwrap();
	service.configure("library-sync", true, 3600).await.unwrap();
	SqlxScheduleRepo::new(db.clone())
		.advance("library-sync", chrono::Utc::now(), chrono::Utc::now())
		.await
		.unwrap();

	jobs.enqueue_detached(JobType::LibrarySync).await.unwrap();
	assert_eq!(service.run_due().await.unwrap(), 0);
	assert_eq!(library_sync_jobs(&jobs).await, 1);
}

#[tokio::test]
async fn configure_rejects_non_positive_interval() {
	let db = memory_db().await;
	let (service, _) = scheduler(&db);
	service.ensure_defaults().await.unwrap();

	assert!(matches!(
		service.configure("library-sync", true, 0).await,
		Err(Error::Validation(_))
	));
}
