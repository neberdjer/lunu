use super::builders::*;
use super::*;

#[tokio::test]
async fn double_approve_enqueues_one_grab() {
	let db = memory_db().await;
	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let requests = request_service(&db, jobs.clone());
	SqlxRequestRepo::new(db.clone())
		.create(&hobbit())
		.await
		.unwrap();

	requests.approve("admin", "r1").await.unwrap();
	assert!(requests.approve("admin", "r1").await.is_err());

	let grabs = jobs
		.list()
		.await
		.unwrap()
		.into_iter()
		.filter(|job| job.job_type == JobType::Grab)
		.count();
	assert_eq!(grabs, 1);
}
