use lunu_core::models::{Media, Placement};
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
		Arc::new(RecordingSidecar::default()),
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

async fn imports_with_sidecar(db: &Db) -> (ImportService, Arc<RecordingSidecar>) {
	seed_download(db, Utc::now()).await;
	let (imports, probes) = imports_probed(db, Arc::new(FakeMerger::new(false))).await;
	(imports, probes.sidecar)
}

#[tokio::test]
async fn an_import_leaves_audiobookshelf_a_metadata_file_beside_the_audio() {
	let db = memory_db().await;
	let (imports, sidecar) = imports_with_sidecar(&db).await;

	imports.import("d1", "/downloads/The Hobbit").await.unwrap();

	let written = sidecar.written.lock().unwrap().clone();
	assert_eq!(written.len(), 1, "exactly one sidecar per import");
	let (dir, opf, cover) = &written[0];
	assert_eq!(
		dir, "/library/Unknown Author/The Hobbit",
		"the sidecar belongs in the item folder audiobookshelf scans"
	);
	assert!(opf.contains("<dc:title>The Hobbit</dc:title>"));
	assert!(
		opf.contains("opf:scheme=\"ASIN\">B01</dc:identifier>"),
		"the asin is the whole point: it outranks a folder name guess, got {opf}"
	);
	assert_eq!(
		cover.as_deref(),
		Some("https://covers/hobbit.jpg"),
		"the cover url must reach the writer, which is what makes cover.jpg possible"
	);
}

#[tokio::test]
async fn turning_the_setting_off_leaves_the_library_untouched() {
	let db = memory_db().await;
	let (imports, sidecar) = imports_with_sidecar(&db).await;
	settings_service(&db)
		.set("import_write_metadata", "off")
		.await
		.unwrap();

	imports.import("d1", "/downloads/The Hobbit").await.unwrap();

	assert!(
		sidecar.written.lock().unwrap().is_empty(),
		"an operator who manages metadata elsewhere must not have files written under them"
	);
}

#[tokio::test]
async fn the_configured_keep_list_reaches_the_importer() {
	let db = memory_db().await;
	seed_download(&db, Utc::now()).await;
	let settings = settings_service(&db);
	settings.set("import_keep_extensions", "jpg").await.unwrap();
	settings
		.set("import_unlisted_files", "extras")
		.await
		.unwrap();
	let (imports, probes) = imports_probed(&db, Arc::new(FakeMerger::new(false))).await;

	imports.import("d1", "/downloads/The Hobbit").await.unwrap();

	let filter = probes.importer.filter.lock().unwrap().clone().unwrap();
	assert_eq!(
		placed(&filter, "back.jpg"),
		Placement::Library,
		"the configured keep list must reach the importer"
	);
	assert_eq!(
		placed(&filter, "cover.jpg"),
		Placement::Skip,
		"cover.jpg is reserved while lunu is writing its own"
	);
	assert_eq!(
		placed(&filter, "tracker.nfo"),
		Placement::Extras,
		"the operator's choice for unlisted files must reach the importer, not a default"
	);
	assert_eq!(
		placed(&filter, "01.mp3"),
		Placement::Library,
		"audio is never filtered out"
	);
}

#[tokio::test]
async fn an_unset_keep_list_falls_back_to_the_registry_default() {
	let db = memory_db().await;
	seed_download(&db, Utc::now()).await;
	let (imports, probes) = imports_probed(&db, Arc::new(FakeMerger::new(false))).await;

	imports.import("d1", "/downloads/The Hobbit").await.unwrap();

	let filter = probes.importer.filter.lock().unwrap().clone().unwrap();
	assert_eq!(
		placed(&filter, "book.pdf"),
		Placement::Library,
		"book documents are in the shipped default keep list"
	);
	assert_eq!(
		placed(&filter, "tracker.nfo"),
		Placement::Skip,
		"with nothing configured the default is to leave tracker litter behind"
	);
	assert_eq!(
		placed(&filter, "metadata.opf"),
		Placement::Skip,
		"lunu writes the sidecar itself, so linking the release copy would clobber the source"
	);
}

fn placed(filter: &lunu_core::models::ImportFilter, name: &str) -> Placement {
	filter.placement(std::path::Path::new(name))
}
