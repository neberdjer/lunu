use super::*;

struct RecordingNotifier {
	id: &'static str,
	events: std::sync::Mutex<Vec<String>>,
}

impl RecordingNotifier {
	fn named(id: &'static str) -> Self {
		Self {
			id,
			events: std::sync::Mutex::new(Vec::new()),
		}
	}
}

#[async_trait]
impl Notifier for RecordingNotifier {
	fn id(&self) -> &'static str {
		self.id
	}
	async fn deliver(&self, event: &NotificationEvent) -> CoreResult<()> {
		self.events.lock().unwrap().push(event.message());
		Ok(())
	}
}

#[tokio::test]
async fn notification_service_dispatches_to_every_notifier() {
	let db = memory_db().await;
	let a = Arc::new(RecordingNotifier::named("a"));
	let b = Arc::new(RecordingNotifier::named("b"));
	let service = NotificationService::new(
		vec![a.clone(), b.clone()],
		Arc::new(SqlxNotificationDeliveryRepo::new(db.clone())),
	);

	let event = NotificationEvent {
		kind: NotificationKind::RequestAvailable,
		request_id: "r1".to_string(),
		title: "Dune".to_string(),
		user_id: "u1".to_string(),
	};
	service.dispatch("job-1", &event).await.unwrap();

	assert_eq!(
		a.events.lock().unwrap().as_slice(),
		&["Now available: Dune"]
	);
	assert_eq!(
		b.events.lock().unwrap().as_slice(),
		&["Now available: Dune"]
	);
}

struct FlakyNotifier {
	id: &'static str,
	calls: std::sync::atomic::AtomicUsize,
	fail_until: usize,
}

impl FlakyNotifier {
	fn new(id: &'static str, fail_until: usize) -> Self {
		Self {
			id,
			calls: std::sync::atomic::AtomicUsize::new(0),
			fail_until,
		}
	}
	fn calls(&self) -> usize {
		self.calls.load(std::sync::atomic::Ordering::Relaxed)
	}
}

#[async_trait]
impl Notifier for FlakyNotifier {
	fn id(&self) -> &'static str {
		self.id
	}
	async fn deliver(&self, _event: &NotificationEvent) -> CoreResult<()> {
		let call = self
			.calls
			.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
		if call < self.fail_until {
			Err(Error::Integration("channel is down".to_string()))
		} else {
			Ok(())
		}
	}
}

#[tokio::test]
async fn a_retry_only_re_runs_the_channel_that_failed() {
	let db = memory_db().await;
	let steady = Arc::new(FlakyNotifier::new("steady", 0));
	let flaky = Arc::new(FlakyNotifier::new("flaky", 1));
	let service = NotificationService::new(
		vec![steady.clone(), flaky.clone()],
		Arc::new(SqlxNotificationDeliveryRepo::new(db.clone())),
	);
	let event = NotificationEvent {
		kind: NotificationKind::RequestAvailable,
		request_id: "r1".to_string(),
		title: "Dune".to_string(),
		user_id: "u1".to_string(),
	};

	let first = service.dispatch("job-1", &event).await.unwrap();
	assert_eq!(
		first.failed, 1,
		"the flaky channel fails on the first attempt"
	);

	let second = service.dispatch("job-1", &event).await.unwrap();
	assert_eq!(second.failed, 0, "the retry completes the delivery");

	assert_eq!(
		steady.calls(),
		1,
		"the already-delivered channel is not re-sent on retry"
	);
	assert_eq!(flaky.calls(), 2, "only the failed channel is re-attempted");
}

#[tokio::test]
async fn pruning_reaps_delivery_rows_for_jobs_that_no_longer_exist() {
	let db = memory_db().await;
	let jobs = JobService::new(Arc::new(SqlxJobRepo::new(db.clone())));
	let live = jobs
		.enqueue_for(JobType::Notify, "payload", "r1")
		.await
		.unwrap();

	let deliveries = SqlxNotificationDeliveryRepo::new(db.clone());
	deliveries.record(&live.id, "email").await.unwrap();
	deliveries
		.record("job-that-was-cancelled", "email")
		.await
		.unwrap();

	let service = NotificationService::new(
		vec![],
		Arc::new(SqlxNotificationDeliveryRepo::new(db.clone())),
	);
	assert_eq!(service.prune_orphaned_deliveries().await.unwrap(), 1);

	assert_eq!(
		deliveries.delivered_channels(&live.id).await.unwrap(),
		vec!["email".to_string()],
		"the live job's records are kept"
	);
	assert!(
		deliveries
			.delivered_channels("job-that-was-cancelled")
			.await
			.unwrap()
			.is_empty(),
		"the orphaned job's records are reaped"
	);
}
