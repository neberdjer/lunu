use super::*;

#[tokio::test]
async fn cancelling_a_finished_torrent_leaves_it_seeding_too() {
	let db = memory_db().await;
	seed_approved_request(&db).await;
	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let client = Arc::new(StubClient::torrent());
	let grabs = grab_service(&db, jobs, client.clone());
	grabs
		.grab("r1", Some(selection(Some("abc"))))
		.await
		.unwrap();
	let downloads = SqlxDownloadRepo::new(db.clone());
	let download = downloads.find_by_request("r1").await.unwrap().unwrap();
	downloads
		.update_status(&download.id, DownloadState::Completed, 100, Utc::now())
		.await
		.unwrap();

	grabs.cancel("r1").await.unwrap();

	assert!(
		client.removals.lock().unwrap().is_empty(),
		"a user cancelling must not earn a hit and run either, so the seeding rule has to hold \
		 on every removal path, not just the automatic one"
	);
}
