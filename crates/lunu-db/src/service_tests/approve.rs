use super::builders::*;
use super::*;

#[tokio::test]
async fn double_approve_enqueues_one_grab() {
	let db = memory_db().await;
	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let requests = request_service(&db, jobs.clone());
	let now = Utc::now();
	SqlxRequestRepo::new(db.clone())
		.create(&Request {
			work_id: "work-B01".to_string(),
			format: Format::Audiobook,
			id: "r1".to_string(),
			user_id: "u1".to_string(),
			asin: Some("B01".to_string()),
			title: "The Hobbit".to_string(),
			author: None,
			cover_url: None,
			status: RequestStatus::Pending,
			approved_by: None,
			notes: None,
			quality_profile_id: None,
			created_at: now,
			updated_at: now,
		})
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
