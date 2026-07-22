use lunu_core::models::MergeState;
use lunu_core::repo::MediaRepo;

use crate::service_tests::builders::*;
use crate::service_tests::*;

async fn events_for(db: &Db, media_id: &str) -> Vec<(String, Option<String>)> {
	activity_service(db)
		.list_page(50, 0)
		.await
		.unwrap()
		.into_iter()
		.filter(|entry| entry.media_id.as_deref() == Some(media_id))
		.map(|entry| (entry.event, entry.detail))
		.collect()
}

#[tokio::test]
async fn a_skipped_merge_explains_itself_instead_of_succeeding_silently() {
	let db = memory_db().await;
	let merger = Arc::new(FakeMerger::skipping("not-multi-file"));
	let media = imported_media(&db, merger.clone()).await;

	merges_for(&db, merger).merge(&media.id).await.unwrap();

	let after = SqlxMediaRepo::new(db.clone())
		.find_by_id(&media.id)
		.await
		.unwrap()
		.unwrap();
	assert_eq!(after.merge_state, MergeState::Skipped);
	assert_eq!(
		after.merge_detail.as_deref(),
		Some("not-multi-file"),
		"a job that changes nothing must say why, or it reads as a silent success"
	);
	assert_eq!(
		events_for(&db, &media.id).await,
		vec![(
			"merge-skipped".to_string(),
			Some("not-multi-file".to_string())
		)],
		"the skip belongs in the activity feed too"
	);
}

#[tokio::test]
async fn a_requested_merge_is_visible_as_queued_before_the_worker_picks_it_up() {
	let db = memory_db().await;
	let merger = Arc::new(FakeMerger::new(true));
	let media = imported_media(&db, merger.clone()).await;
	let media_repo = SqlxMediaRepo::new(db.clone());

	merges_for(&db, merger).request(&media.id).await.unwrap();

	assert_eq!(
		media_repo
			.find_by_id(&media.id)
			.await
			.unwrap()
			.unwrap()
			.merge_state,
		MergeState::Queued,
		"the row must show the merge is in flight, not look untouched"
	);
	assert_eq!(
		mergeable_count(&db).await,
		0,
		"an already queued item must not be offered for merging a second time"
	);
}

#[tokio::test]
async fn a_merge_records_where_it_shelved_the_originals_so_it_can_be_undone() {
	let db = memory_db().await;
	let merger = Arc::new(FakeMerger::new(true));
	let media = imported_media(&db, merger.clone()).await;
	let merges = merges_for(&db, merger.clone());
	let media_repo = SqlxMediaRepo::new(db.clone());

	merges.merge(&media.id).await.unwrap();
	let merged = media_repo.find_by_id(&media.id).await.unwrap().unwrap();
	assert_eq!(merged.merge_state, MergeState::Merged);
	assert_eq!(merged.merge_backup_path.as_deref(), Some("/backup"));

	let restored = merges.revert(&media.id).await.unwrap();
	assert_eq!(restored, 2);
	assert_eq!(
		merger.reverted.lock().unwrap().as_slice(),
		["/library/Unknown Author/The Hobbit/The Hobbit.m4b"],
		"revert must target the file the merge actually produced"
	);

	let reverted = media_repo.find_by_id(&media.id).await.unwrap().unwrap();
	assert_eq!(reverted.merge_state, MergeState::Idle);
	assert!(reverted.merged_path.is_none());
	assert!(reverted.merge_backup_path.is_none());
	assert_eq!(
		mergeable_count(&db).await,
		1,
		"a reverted item is a merge candidate again"
	);
}

#[tokio::test]
async fn there_is_nothing_to_revert_when_the_originals_were_never_shelved() {
	let db = memory_db().await;
	let merger = Arc::new(FakeMerger::new(true));
	let media = imported_media(&db, merger.clone()).await;
	settings_service(&db)
		.set("merge_source_action", "delete")
		.await
		.unwrap();
	let merges = merges_for(&db, merger);

	merges.merge(&media.id).await.unwrap();

	assert!(
		matches!(merges.revert(&media.id).await, Err(Error::Validation(reason)) if reason == "merge-nothing-to-revert"),
		"deleted sources cannot come back, and pretending otherwise would lose the m4b too"
	);
}

#[tokio::test]
async fn a_skipped_item_stops_being_offered_but_can_still_be_merged_by_hand() {
	let db = memory_db().await;
	let merger = Arc::new(FakeMerger::skipping("not-multi-file"));
	let media = imported_media(&db, merger.clone()).await;
	let merges = merges_for(&db, merger);

	merges.merge(&media.id).await.unwrap();

	assert_eq!(
		mergeable_count(&db).await,
		0,
		"merge-all must not re-queue a book it already knows it cannot merge, or every run \
		 would flood the queue with the same no-op jobs"
	);
	assert!(
		merges.request(&media.id).await.is_ok(),
		"an explicit per-item merge is still allowed, since the user may have fixed the cause"
	);
}

#[tokio::test]
async fn a_preview_answers_the_questions_a_merge_would_otherwise_answer_too_late() {
	let db = memory_db().await;
	let merger = Arc::new(FakeMerger::skipping("not-multi-file"));
	let media = imported_media(&db, merger.clone()).await;

	let preview = merges_for(&db, merger).preview(&media.id).await.unwrap();

	assert_eq!(
		preview.skip,
		Some("not-multi-file"),
		"a preview must say the merge would do nothing before the user commits to it"
	);
	assert_eq!(
		preview.output_path, "/library/Unknown Author/The Hobbit/The Hobbit.m4b",
		"the preview names the exact file the merge would write"
	);
	assert!(
		matches!(preview.sources, lunu_core::models::SourceDisposition::Move { backup } if backup == "/backup"),
		"the preview must show what would become of the originals, which is the destructive part"
	);
}

#[tokio::test]
async fn a_preview_refuses_the_same_misconfiguration_a_merge_would() {
	let db = memory_db().await;
	let merger = Arc::new(FakeMerger::new(true));
	let (imports, _) = imports_with_merge(&db, merger.clone()).await;
	imports.import("d1", "/downloads/The Hobbit").await.unwrap();
	let media = media_of_request(&db).await;

	assert!(
		matches!(merges_for(&db, merger).preview(&media.id).await, Err(Error::Validation(reason)) if reason == "merge-backup-not-configured"),
		"a preview that hid the misconfiguration would send the user to click merge and fail there"
	);
}

#[tokio::test]
async fn each_merge_transition_is_announced_live() {
	let db = memory_db().await;
	let merger = Arc::new(FakeMerger::new(true));
	let media = imported_media(&db, merger.clone()).await;
	let events = Arc::new(MergeEvents::default());
	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let merges = merge_service_with(&db, jobs, merger, events.clone());

	merges.request(&media.id).await.unwrap();
	merges.merge(&media.id).await.unwrap();
	merges.revert(&media.id).await.unwrap();

	let seen = events.seen.lock().unwrap().clone();
	assert_eq!(
		seen,
		vec![
			(media.id.clone(), "queued".to_string(), None),
			(
				media.id.clone(),
				"merged".to_string(),
				Some("/library/Unknown Author/The Hobbit/The Hobbit.m4b".to_string())
			),
			(media.id.clone(), "idle".to_string(), None),
		],
		"a client watching the socket must see the row move without refetching it"
	);
}

#[tokio::test]
async fn asking_twice_for_the_same_merge_yields_one_job() {
	let db = memory_db().await;
	let merger = Arc::new(FakeMerger::new(true));
	let media = imported_media(&db, merger.clone()).await;
	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let merges = merge_service(&db, jobs.clone(), merger);

	let first = merges.request(&media.id).await.unwrap();
	let second = merges.request(&media.id).await.unwrap();

	assert_eq!(
		first, second,
		"a second request must join the queued job, not start a rival one"
	);
	let queued: Vec<_> = jobs
		.list()
		.await
		.unwrap()
		.into_iter()
		.filter(|job| job.job_type == JobType::Merge)
		.collect();
	assert_eq!(
		queued.len(),
		1,
		"two merge jobs for one book would transcode into the same part file"
	);
}
