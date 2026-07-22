use super::builders::*;
use super::monitor::FakeClient;
use super::*;

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
