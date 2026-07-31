use super::builders::*;
use super::monitor::FakeClient;
use super::*;
use lunu_core::services::ClientRoster;

async fn poll_failed_at(db: &Db, progress: f64) -> Arc<FakeClient> {
	seed_download(db, Utc::now()).await;
	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let client = Arc::new(FakeClient::responding(Some(DownloadStatus {
		state: DownloadState::Failed,
		progress,
		content_path: None,
	})));
	super::monitor::monitor_with(db, jobs, client.clone())
		.poll(&MonitorPayload {
			download_id: "d1".to_string(),
			misses: 0,
			stalls: 0,
		})
		.await
		.unwrap();
	client
}

#[tokio::test]
async fn a_finished_torrent_is_left_seeding_even_when_the_request_fails() {
	let db = memory_db().await;

	let client = poll_failed_at(&db, 1.0).await;

	assert!(
		client.removals().is_empty(),
		"removing a torrent that already hit 100 percent is what earns a hit and run on a \
		 private tracker, so a failure after completion must leave it seeding"
	);
}

#[tokio::test]
async fn an_unfinished_torrent_is_still_removed_since_it_owes_no_seeding() {
	let db = memory_db().await;

	let client = poll_failed_at(&db, 0.4).await;

	assert_eq!(
		client.removals(),
		vec![("abc".to_string(), true)],
		"a partial download carries no seeding obligation, so leaving it would only waste disk"
	);
}

#[tokio::test]
async fn an_operator_can_turn_client_removal_off_entirely() {
	let db = memory_db().await;
	settings_service(&db)
		.set("download_remove_failed", "off")
		.await
		.unwrap();

	let client = poll_failed_at(&db, 0.4).await;

	assert!(
		client.removals().is_empty(),
		"a tracker with strict rules must be able to opt out of automatic removal"
	);
}

#[tokio::test]
async fn a_stale_monitor_cannot_fail_a_request_that_was_re_attempted() {
	let db = memory_db().await;
	let base = Utc::now();
	seed_download(&db, base).await;
	let downloads = SqlxDownloadRepo::new(db.clone());

	downloads
		.update_status(
			"d1",
			DownloadState::Failed,
			10,
			base + chrono::Duration::seconds(30),
		)
		.await
		.unwrap();

	downloads
		.create(&Download {
			id: "d2".to_string(),
			request_id: "r1".to_string(),
			client: "qbittorrent".to_string(),
			category: "lunu".to_string(),
			release_title: "The Hobbit [M4B]".to_string(),
			indexer: "MyTracker".to_string(),
			download_url: "magnet:?xt=urn:btih:def".to_string(),
			client_ref: Some("def".to_string()),
			state: DownloadState::Downloading,
			progress: 5,
			created_at: base + chrono::Duration::seconds(60),
			updated_at: base + chrono::Duration::seconds(60),
		})
		.await
		.unwrap();

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let client = Arc::new(FakeClient::responding(Some(DownloadStatus {
		state: DownloadState::Failed,
		progress: 0.05,
		content_path: None,
	})));
	let monitor = MonitorService::new(
		Arc::new(SqlxDownloadRepo::new(db.clone())),
		ClientRoster::new(vec![client]),
		request_service(&db, jobs.clone()),
		jobs.clone(),
		Arc::new(NoopPublisher),
		settings_service(&db),
	);

	// the stale d1 monitor fires and sees a failure
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
		RequestStatus::Downloading,
		"a monitor for a superseded download must not fail the request its retry is fulfilling"
	);
}
