use lunu_core::models::Media;
use lunu_core::repo::MediaRepo;

use super::builders::*;
use super::*;

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
		request_service(&db, jobs.clone()),
		settings,
		importer.clone(),
		Arc::new(MediaService::new(Arc::new(SqlxMediaRepo::new(db.clone())))),
		merge_service(&db, jobs, Arc::new(FakeMerger::new(false))),
	);

	imports.import("d1", "/downloads/The Hobbit").await.unwrap();

	let call = importer.call.lock().unwrap().clone().unwrap();
	assert_eq!(call.0, "/downloads/The Hobbit");
	assert_eq!(call.1, "/library/Unknown Author/The Hobbit");
	assert_eq!(request_status(&db).await, RequestStatus::Available);
}

#[tokio::test]
async fn import_retry_does_not_clobber_admin_curated_media() {
	let db = memory_db().await;
	seed_download(&db, Utc::now()).await;
	seed_media_for_request(&db, true).await;

	import_service_for(&db)
		.await
		.import("d1", "/downloads/The Hobbit")
		.await
		.unwrap();

	let media = SqlxMediaRepo::new(db.clone())
		.find_by_id("m1")
		.await
		.unwrap()
		.unwrap();
	assert_eq!(
		media.title, "The Hobbit (Andy Serkis Edition)",
		"an admin's curated title must survive an import retry"
	);
	assert_eq!(media.asin.as_deref(), Some("CURATED"));
	assert_eq!(media.cover_url.as_deref(), Some("curated-cover"));
	assert_eq!(media.series_name.as_deref(), Some("Middle-earth"));
	assert!(media.overridden);
	assert_eq!(
		SqlxMediaRepo::new(db.clone()).count().await.unwrap(),
		1,
		"the retry must not insert a duplicate media row"
	);
	assert_eq!(
		request_status(&db).await,
		RequestStatus::Available,
		"the import still completes"
	);
}

#[tokio::test]
async fn import_updates_media_when_not_overridden() {
	let db = memory_db().await;
	seed_download(&db, Utc::now()).await;
	seed_media_for_request(&db, false).await;

	import_service_for(&db)
		.await
		.import("d1", "/downloads/The Hobbit")
		.await
		.unwrap();

	let media = SqlxMediaRepo::new(db.clone())
		.find_by_id("m1")
		.await
		.unwrap()
		.unwrap();
	assert_eq!(
		media.title, "The Hobbit",
		"an uncurated row must still be refreshed from the request"
	);
	assert_eq!(media.library_path, "/library/Unknown Author/The Hobbit");
	assert_eq!(
		SqlxMediaRepo::new(db.clone()).count().await.unwrap(),
		1,
		"the update must not insert a duplicate media row"
	);
}

async fn import_service_for(db: &Db) -> ImportService {
	imports_with(db, Arc::new(FakeMerger::new(false))).await.0
}

async fn seed_media_for_request(db: &Db, overridden: bool) {
	SqlxMediaRepo::new(db.clone())
		.insert(&Media {
			work_id: Some("work-CURATED".to_string()),
			asin: Some("CURATED".to_string()),
			title: "The Hobbit (Andy Serkis Edition)".to_string(),
			author: Some("J.R.R. Tolkien".to_string()),
			cover_url: Some("curated-cover".to_string()),
			series_name: Some("Middle-earth".to_string()),
			series_sequence: Some("1".to_string()),
			library_path: "/library/Tolkien/The Hobbit".to_string(),
			overridden,
			request_id: Some("r1".to_string()),
			..media("m1")
		})
		.await
		.unwrap();
}
