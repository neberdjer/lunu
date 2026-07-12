use super::builders::*;
use super::*;

#[tokio::test]
async fn approving_a_request_enqueues_a_grab_job() {
	let db = memory_db().await;

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let requests = request_service(&db, jobs.clone());

	let now = Utc::now();
	let request = Request {
		id: "r1".to_string(),
		user_id: "u1".to_string(),
		asin: "B01".to_string(),
		title: "The Hobbit".to_string(),
		author: None,
		cover_url: None,
		status: RequestStatus::Pending,
		approved_by: None,
		notes: None,
		quality_profile_id: None,
		created_at: now,
		updated_at: now,
	};
	SqlxRequestRepo::new(db.clone())
		.create(&request)
		.await
		.unwrap();

	let approved = requests.approve("admin", "r1").await.unwrap();
	assert_eq!(approved.status, RequestStatus::Approved);

	let listed = jobs.list().await.unwrap();
	let grabs: Vec<_> = listed
		.iter()
		.filter(|job| job.job_type == JobType::Grab)
		.collect();
	assert_eq!(grabs.len(), 1);
	assert!(grabs[0].payload.contains("r1"));
	assert!(listed.iter().any(|job| job.job_type == JobType::Notify));
}

#[tokio::test]
async fn marking_available_enqueues_a_notification() {
	let db = memory_db().await;
	let now = Utc::now();
	SqlxRequestRepo::new(db.clone())
		.create(&Request {
			id: "r1".to_string(),
			user_id: "u1".to_string(),
			asin: "B01".to_string(),
			title: "The Hobbit".to_string(),
			author: None,
			cover_url: None,
			status: RequestStatus::Importing,
			approved_by: None,
			notes: None,
			quality_profile_id: None,
			created_at: now,
			updated_at: now,
		})
		.await
		.unwrap();
	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let requests = request_service(&db, jobs.clone());

	requests.mark_available("r1").await.unwrap();

	let listed = jobs.list().await.unwrap();
	let notifies: Vec<_> = listed
		.iter()
		.filter(|job| job.job_type == JobType::Notify)
		.collect();
	assert_eq!(notifies.len(), 1);
	assert!(notifies[0].payload.contains("request-available"));
	assert!(notifies[0].payload.contains("The Hobbit"));
}

#[derive(Default)]
struct RecordingNotifier {
	events: std::sync::Mutex<Vec<String>>,
}

#[async_trait]
impl Notifier for RecordingNotifier {
	fn id(&self) -> &'static str {
		"recording"
	}
	async fn deliver(&self, event: &NotificationEvent) -> CoreResult<()> {
		self.events.lock().unwrap().push(event.message());
		Ok(())
	}
}

#[tokio::test]
async fn notification_service_dispatches_to_every_notifier() {
	let a = Arc::new(RecordingNotifier::default());
	let b = Arc::new(RecordingNotifier::default());
	let service = NotificationService::new(vec![a.clone(), b.clone()]);

	let event = NotificationEvent {
		kind: NotificationKind::RequestAvailable,
		request_id: "r1".to_string(),
		title: "Dune".to_string(),
		user_id: "u1".to_string(),
	};
	service.dispatch(&event).await.unwrap();

	assert_eq!(
		a.events.lock().unwrap().as_slice(),
		&["Now available: Dune"]
	);
	assert_eq!(
		b.events.lock().unwrap().as_slice(),
		&["Now available: Dune"]
	);
}

#[derive(Default)]
struct FakeImporter {
	call: std::sync::Mutex<Option<(String, String)>>,
}

#[async_trait]
impl Importer for FakeImporter {
	async fn import(&self, source: &str, destination: &str) -> CoreResult<()> {
		*self.call.lock().unwrap() = Some((source.to_string(), destination.to_string()));
		Ok(())
	}
}

#[tokio::test]
async fn import_places_content_and_marks_available() {
	let db = memory_db().await;
	seed_download(&db, Utc::now()).await;

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let settings = settings_service(&db);
	settings.set("library_dir", "/library").await.unwrap();
	let importer = Arc::new(FakeImporter::default());
	let imports = ImportService::new(
		Arc::new(SqlxDownloadRepo::new(db.clone())),
		request_service(&db, jobs),
		settings,
		importer.clone(),
		Arc::new(MediaService::new(Arc::new(SqlxMediaRepo::new(db.clone())))),
	);

	imports.import("d1", "/downloads/The Hobbit").await.unwrap();

	let call = importer.call.lock().unwrap().clone().unwrap();
	assert_eq!(call.0, "/downloads/The Hobbit");
	assert_eq!(call.1, "/library/Unknown Author/The Hobbit");
	assert_eq!(
		SqlxRequestRepo::new(db.clone())
			.find_by_id("r1")
			.await
			.unwrap()
			.unwrap()
			.status,
		RequestStatus::Available
	);
}

#[tokio::test]
async fn request_transitions_record_activity() {
	let db = memory_db().await;

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let activity = activity_service(&db);
	let requests = request_service_with_activity(&db, jobs, activity.clone());

	let now = Utc::now();
	let request = Request {
		id: "r1".to_string(),
		user_id: "u1".to_string(),
		asin: "B01".to_string(),
		title: "The Hobbit".to_string(),
		author: None,
		cover_url: None,
		status: RequestStatus::Pending,
		approved_by: None,
		notes: None,
		quality_profile_id: None,
		created_at: now,
		updated_at: now,
	};
	SqlxRequestRepo::new(db.clone())
		.create(&request)
		.await
		.unwrap();

	requests.approve("admin", "r1").await.unwrap();
	requests.mark_downloading("r1").await.unwrap();

	let events: Vec<String> = activity
		.for_request("r1")
		.await
		.unwrap()
		.into_iter()
		.map(|entry| entry.event)
		.collect();
	assert!(events.contains(&"approved".to_string()));
	assert!(events.contains(&"downloading".to_string()));
	assert_eq!(activity.list_page(10, 0).await.unwrap().len(), 2);
}

#[tokio::test]
async fn import_requires_library_configured() {
	let db = memory_db().await;
	seed_download(&db, Utc::now()).await;

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let imports = ImportService::new(
		Arc::new(SqlxDownloadRepo::new(db.clone())),
		request_service(&db, jobs),
		settings_service(&db),
		Arc::new(FakeImporter::default()),
		Arc::new(MediaService::new(Arc::new(SqlxMediaRepo::new(db.clone())))),
	);

	assert!(imports.import("d1", "/downloads/x").await.is_err());
}

#[derive(Default)]
struct RecordingPublisher {
	events: std::sync::Mutex<Vec<String>>,
}

impl EventPublisher for RecordingPublisher {
	fn publish(&self, event: &lunu_core::models::LiveEvent) {
		if let lunu_core::models::LiveEvent::Activity(activity) = event {
			self.events
				.lock()
				.unwrap()
				.push(format!("{}:{}", activity.request_id, activity.event));
		}
	}
}

#[tokio::test]
async fn recording_activity_publishes_event() {
	let db = memory_db().await;
	let publisher = Arc::new(RecordingPublisher::default());
	let activity = ActivityService::new(
		Arc::new(SqlxActivityRepo::new(db.clone())),
		publisher.clone(),
	);

	activity
		.record("r1", "downloading", None, None)
		.await
		.unwrap();

	assert_eq!(
		publisher.events.lock().unwrap().as_slice(),
		&["r1:downloading".to_string()]
	);
}
