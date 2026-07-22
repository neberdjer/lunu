mod state;

use super::builders::*;
use super::*;

async fn merge_jobs(jobs: &JobService) -> Vec<lunu_core::models::Job> {
	jobs.list()
		.await
		.unwrap()
		.into_iter()
		.filter(|job| job.job_type == JobType::Merge)
		.collect()
}

#[tokio::test]
async fn an_enabled_merge_is_queued_once_the_import_lands() {
	let db = memory_db().await;
	let (imports, jobs) = imports_with_merge(&db, Arc::new(FakeMerger::new(true))).await;

	imports.import("d1", "/downloads/The Hobbit").await.unwrap();

	let queued = merge_jobs(&jobs).await;
	assert_eq!(queued.len(), 1, "an enabled merge follows the import");
	let media = media_of_request(&db).await;
	assert!(
		queued[0].payload.contains(&media.id),
		"the merge job must name the media it was imported as"
	);
	assert_eq!(
		request_status(&db).await,
		RequestStatus::Available,
		"the request is available the moment it imports, not once it merges"
	);
}

#[tokio::test]
async fn an_import_still_succeeds_when_ffmpeg_is_missing() {
	let db = memory_db().await;
	let (imports, jobs) = imports_with_merge(&db, Arc::new(FakeMerger::new(false))).await;

	imports.import("d1", "/downloads/The Hobbit").await.unwrap();

	assert!(
		merge_jobs(&jobs).await.is_empty(),
		"nothing to queue without ffmpeg"
	);
	assert_eq!(
		request_status(&db).await,
		RequestStatus::Available,
		"a missing merger must never hold back a delivered book"
	);
}

#[tokio::test]
async fn merging_an_already_imported_item_plans_an_m4b_in_its_library_directory() {
	let db = memory_db().await;
	let merger = Arc::new(FakeMerger::new(true));
	let (imports, _) = imports_with_merge(&db, merger.clone()).await;
	settings_service(&db)
		.set("merge_backup_dir", "/backup")
		.await
		.unwrap();
	imports.import("d1", "/downloads/The Hobbit").await.unwrap();
	let media = media_of_request(&db).await;

	let merges = merges_for(&db, merger.clone());
	let lunu_core::traits::MergeOutcome::Merged(summary) = merges.merge(&media.id).await.unwrap()
	else {
		panic!("the fake merger always merges");
	};

	assert_eq!(
		summary.output_path, "/library/Unknown Author/The Hobbit/The Hobbit.m4b",
		"the merged file lands inside the item's own library directory"
	);
	assert_eq!(merger.plans.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn moving_sources_is_refused_until_a_backup_directory_exists() {
	let db = memory_db().await;
	let merger = Arc::new(FakeMerger::new(true));
	let (imports, _) = imports_with_merge(&db, merger.clone()).await;
	imports.import("d1", "/downloads/The Hobbit").await.unwrap();
	let media = media_of_request(&db).await;

	let merges = merges_for(&db, merger.clone());
	assert!(
		matches!(merges.merge(&media.id).await, Err(Error::Validation(reason)) if reason == "merge-backup-not-configured"),
		"the default action shelves originals, so it must refuse rather than quietly keep them \
		 beside the merged file where audiobookshelf would count both"
	);
	assert!(
		merger.plans.lock().unwrap().is_empty(),
		"nothing should reach the merger when the destination for originals is unknown"
	);
}

#[tokio::test]
async fn a_retroactive_merge_is_refused_for_an_unknown_item_and_without_ffmpeg() {
	let db = memory_db().await;
	let (imports, _) = imports_with_merge(&db, Arc::new(FakeMerger::new(true))).await;
	imports.import("d1", "/downloads/The Hobbit").await.unwrap();
	let media = media_of_request(&db).await;

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let present = merge_service(&db, jobs.clone(), Arc::new(FakeMerger::new(true)));
	assert!(matches!(
		present.request("nope").await,
		Err(Error::NotFound(_))
	));

	let without = merge_service(&db, jobs, Arc::new(FakeMerger::new(false)));
	assert!(
		matches!(without.request(&media.id).await, Err(Error::Validation(reason)) if reason == "merge-unavailable"),
		"asking to merge without ffmpeg must say so rather than queue a job that cannot run"
	);
}

#[tokio::test]
async fn moving_without_a_backup_directory_is_refused_at_save_time() {
	let db = memory_db().await;
	let settings = settings_service(&db);

	assert!(
		matches!(
			settings.set("merge_source_action", "move").await,
			Err(Error::Validation(reason)) if reason == "merge-backup-not-configured"
		),
		"the pair must be refused where the mistake is made, not hours later inside a job"
	);

	settings.set("merge_backup_dir", "/backup").await.unwrap();
	settings.set("merge_source_action", "move").await.unwrap();
	assert!(
		matches!(
			settings.delete("merge_backup_dir").await,
			Err(Error::Validation(reason)) if reason == "merge-backup-not-configured"
		),
		"clearing the destination while move is selected breaks the same pair"
	);
	settings.delete("merge_source_action").await.unwrap();
	assert!(
		matches!(
			settings.delete("merge_backup_dir").await,
			Err(Error::Validation(reason)) if reason == "merge-backup-not-configured"
		),
		"the action falls back to move, so the destination still cannot be cleared"
	);
}

#[tokio::test]
async fn merge_all_queues_every_unmerged_item_and_says_when_it_stopped_short() {
	let db = memory_db().await;
	let merger = Arc::new(FakeMerger::new(true));
	let (imports, _) = imports_with_merge(&db, merger.clone()).await;
	settings_service(&db)
		.set("merge_backup_dir", "/backup")
		.await
		.unwrap();
	imports.import("d1", "/downloads/The Hobbit").await.unwrap();

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let merges = merge_service(&db, jobs.clone(), merger);
	let queued = merges.request_all().await.unwrap();

	assert_eq!(
		queued.queued, 0,
		"the import already queued this one, and merge-all must not queue it twice"
	);
	assert!(!queued.truncated, "one item is nowhere near the cap");
	assert_eq!(
		merge_jobs(&jobs).await.len(),
		1,
		"exactly one merge job exists for one book"
	);
}

#[tokio::test]
async fn a_merged_item_drops_out_of_the_mergeable_list() {
	let db = memory_db().await;
	let merger = Arc::new(FakeMerger::new(true));
	let (imports, _) = imports_with_merge(&db, merger.clone()).await;
	settings_service(&db)
		.set("merge_backup_dir", "/backup")
		.await
		.unwrap();
	imports.import("d1", "/downloads/The Hobbit").await.unwrap();

	let media = media_of_request(&db).await;
	let merges = merges_for(&db, merger);
	merges.fail(&media.id, "ffmpeg died").await.unwrap();
	assert_eq!(
		mergeable_count(&db).await,
		1,
		"a book whose merge failed is a candidate again, not stuck out of the list"
	);

	merges.merge(&media.id).await.unwrap();

	assert_eq!(
		mergeable_count(&db).await,
		0,
		"once merged it must stop being offered"
	);
}

#[tokio::test]
async fn two_books_can_wait_to_be_merged_at_the_same_time() {
	let db = memory_db().await;
	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));

	for media_id in ["m1", "m2"] {
		jobs.enqueue_detached_with(
			JobType::Merge,
			&lunu_core::models::MergePayload {
				media_id: media_id.to_string(),
			},
		)
		.await
		.unwrap();
	}

	assert_eq!(
		merge_jobs(&jobs).await.len(),
		2,
		"merges are detached like the recurring jobs, so the one-per-type index that dedupes \
		 library sync must not also cap the queue at a single book"
	);
}

#[tokio::test]
async fn a_recurring_job_is_still_deduped() {
	let db = memory_db().await;
	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));

	assert!(jobs.enqueue_detached(JobType::LibrarySync).await.unwrap());
	assert!(
		!jobs.enqueue_detached(JobType::LibrarySync).await.unwrap(),
		"a second sync must still collapse into the pending one"
	);
}
