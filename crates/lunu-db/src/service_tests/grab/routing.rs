use super::*;

fn usenet_selection() -> ReleaseSelection {
	ReleaseSelection {
		title: "The Hobbit [M4B]".to_string(),
		indexer: "NZBIndexer".to_string(),
		download_url: "https://indexer/get.nzb".to_string(),
		info_hash: None,
		protocol: Protocol::Usenet,
	}
}

#[tokio::test]
async fn a_usenet_grab_routes_to_the_usenet_client_and_tracks_the_nzo_id() {
	let db = memory_db().await;
	seed_approved_request(&db).await;

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let torrent = Arc::new(StubClient::torrent());
	let nzb = Arc::new(StubClient::usenet());
	let grabs = grab_service_with(&db, jobs.clone(), vec![torrent.clone(), nzb.clone()]);

	grabs.grab("r1", Some(usenet_selection())).await.unwrap();

	assert_eq!(nzb.adds().len(), 1);
	assert!(
		torrent.adds().is_empty(),
		"the torrent client must not see a usenet release"
	);

	let download = SqlxDownloadRepo::new(db.clone())
		.find_by_request("r1")
		.await
		.unwrap()
		.unwrap();
	assert_eq!(download.client, "sabnzbd");
	assert_eq!(
		download.client_ref.as_deref(),
		Some("nzo-1"),
		"an nzb has no info hash, so the client-assigned id is what the monitor polls by"
	);
	assert_eq!(monitor_jobs(&jobs).await.len(), 1);
}

#[tokio::test]
async fn a_usenet_grab_without_a_usenet_client_is_refused() {
	let db = memory_db().await;
	seed_approved_request(&db).await;

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let grabs = grab_service(&db, jobs.clone(), Arc::new(StubClient::torrent()));

	let error = grabs
		.grab("r1", Some(usenet_selection()))
		.await
		.unwrap_err();
	assert!(
		matches!(error, lunu_core::Error::Validation(reason) if reason == "no-client-for-protocol")
	);

	assert!(
		SqlxDownloadRepo::new(db.clone())
			.find_by_request("r1")
			.await
			.unwrap()
			.is_none(),
		"a refused grab must not strand a download row"
	);
}
