use super::*;
use lunu_core::models::Media;
use lunu_core::repo::MediaRepo;

pub(crate) struct FakeMerger {
	pub(crate) available: bool,
	pub(crate) plans: std::sync::Mutex<Vec<String>>,
	pub(crate) reverted: std::sync::Mutex<Vec<String>>,
	pub(crate) skip: Option<&'static str>,
}

impl FakeMerger {
	pub(crate) fn new(available: bool) -> Self {
		Self {
			available,
			plans: std::sync::Mutex::new(Vec::new()),
			reverted: std::sync::Mutex::new(Vec::new()),
			skip: None,
		}
	}

	pub(crate) fn skipping(reason: &'static str) -> Self {
		Self {
			skip: Some(reason),
			..Self::new(true)
		}
	}
}

#[async_trait]
impl lunu_core::traits::Merger for FakeMerger {
	async fn test_connection(&self) -> CoreResult<()> {
		if self.available {
			return Ok(());
		}
		Err(Error::Validation(
			lunu_core::consts::reasons::MERGE_UNAVAILABLE.to_string(),
		))
	}

	async fn merge(
		&self,
		plan: &lunu_core::traits::MergePlan,
	) -> CoreResult<lunu_core::traits::MergeOutcome> {
		self.plans.lock().unwrap().push(plan.output_path.clone());
		if let Some(reason) = self.skip {
			return Ok(lunu_core::traits::MergeOutcome::Skipped(reason));
		}
		let backup = match &plan.sources {
			lunu_core::models::SourceDisposition::Move { backup } => Some(backup.clone()),
			_ => None,
		};
		Ok(lunu_core::traits::MergeOutcome::Merged(
			lunu_core::traits::MergeSummary {
				output_path: plan.output_path.clone(),
				backup_path: backup,
				chapters: 3,
				handled_sources: usize::from(
					plan.sources != lunu_core::models::SourceDisposition::Keep,
				),
			},
		))
	}

	async fn preview(
		&self,
		plan: &lunu_core::traits::MergePlan,
	) -> CoreResult<lunu_core::traits::MergePreview> {
		Ok(lunu_core::traits::MergePreview {
			output_path: plan.output_path.clone(),
			skip: self.skip,
			chapters: vec![lunu_core::traits::PreviewChapter {
				title: "01".to_string(),
				seconds: 60.0,
			}],
			total_seconds: 60.0,
			copyable: false,
			sources: plan.sources.clone(),
			bitrate: plan.bitrate.clone(),
		})
	}

	async fn revert(&self, plan: &lunu_core::traits::RevertPlan) -> CoreResult<usize> {
		self.reverted.lock().unwrap().push(plan.merged_path.clone());
		Ok(2)
	}
}

pub(crate) fn merge_service(
	db: &Db,
	jobs: Arc<lunu_core::services::JobService>,
	merger: Arc<dyn lunu_core::traits::Merger>,
) -> Arc<lunu_core::services::MergeService> {
	merge_service_with(db, jobs, merger, Arc::new(NoopPublisher))
}

#[derive(Default)]
pub(crate) struct MergeEvents {
	pub(crate) seen: std::sync::Mutex<Vec<(String, String, Option<String>)>>,
}

impl lunu_core::traits::EventPublisher for MergeEvents {
	fn publish(&self, event: &lunu_core::models::LiveEvent) {
		if let lunu_core::models::LiveEvent::Merge(media) = event {
			self.seen.lock().unwrap().push((
				media.id.clone(),
				media.merge_state.as_str().to_string(),
				media.merged_path.clone(),
			));
		}
	}
}

pub(crate) fn merge_service_with(
	db: &Db,
	jobs: Arc<lunu_core::services::JobService>,
	merger: Arc<dyn lunu_core::traits::Merger>,
	events: Arc<dyn lunu_core::traits::EventPublisher>,
) -> Arc<lunu_core::services::MergeService> {
	Arc::new(lunu_core::services::MergeService::new(
		Arc::new(SqlxMediaRepo::new(db.clone())),
		settings_service(db),
		merger,
		jobs,
		activity_service(db),
		events,
	))
}

pub(crate) struct ImportProbes {
	pub(crate) importer: Arc<FakeImporter>,
	pub(crate) sidecar: Arc<RecordingSidecar>,
}

pub(crate) async fn imports_probed(
	db: &Db,
	merger: Arc<FakeMerger>,
) -> (ImportService, ImportProbes) {
	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let settings = settings_service(db);
	settings.set("library_dir", "/library").await.unwrap();
	let importer = Arc::new(FakeImporter::default());
	let sidecar = Arc::new(RecordingSidecar::default());
	let imports = ImportService::new(
		Arc::new(SqlxDownloadRepo::new(db.clone())),
		request_service(db, jobs.clone()),
		settings,
		importer.clone(),
		Arc::new(MediaService::new(Arc::new(SqlxMediaRepo::new(db.clone())))),
		merge_service(db, jobs.clone(), merger),
		sidecar.clone(),
	);
	(imports, ImportProbes { importer, sidecar })
}

pub(crate) async fn imports_with(
	db: &Db,
	merger: Arc<FakeMerger>,
) -> (ImportService, Arc<JobService>) {
	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let settings = settings_service(db);
	settings.set("library_dir", "/library").await.unwrap();
	let imports = ImportService::new(
		Arc::new(SqlxDownloadRepo::new(db.clone())),
		request_service(db, jobs.clone()),
		settings,
		Arc::new(FakeImporter::default()),
		Arc::new(MediaService::new(Arc::new(SqlxMediaRepo::new(db.clone())))),
		merge_service(db, jobs.clone(), merger),
		Arc::new(RecordingSidecar::default()),
	);
	(imports, jobs)
}

pub(crate) async fn imports_with_merge(
	db: &Db,
	merger: Arc<FakeMerger>,
) -> (ImportService, Arc<JobService>) {
	seed_download(db, Utc::now()).await;
	settings_service(db)
		.set("merge_enabled", "on")
		.await
		.unwrap();
	imports_with(db, merger).await
}

pub(crate) async fn imported_media(db: &Db, merger: Arc<FakeMerger>) -> Media {
	let (imports, _) = imports_with_merge(db, merger).await;
	settings_service(db)
		.set("merge_backup_dir", "/backup")
		.await
		.unwrap();
	imports.import("d1", "/downloads/The Hobbit").await.unwrap();
	media_of_request(db).await
}

pub(crate) fn merges_for(
	db: &Db,
	merger: Arc<FakeMerger>,
) -> Arc<lunu_core::services::MergeService> {
	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	merge_service(db, jobs, merger)
}

pub(crate) async fn media_of_request(db: &Db) -> Media {
	SqlxMediaRepo::new(db.clone())
		.find_by_request("r1")
		.await
		.unwrap()
		.unwrap()
}

pub(crate) async fn mergeable_count(db: &Db) -> i64 {
	SqlxMediaRepo::new(db.clone())
		.list_count(lunu_core::models::MediaFilter::Mergeable)
		.await
		.unwrap()
}
