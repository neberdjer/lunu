use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use lunu_core::Result as CoreResult;
use lunu_core::consts::crypto::SETTINGS_ENCRYPTION_CONTEXT;
use lunu_core::consts::download::MONITOR_MAX_MISSES;
use lunu_core::crypto::Encryptor;
use lunu_core::models::{
	Book, Chapters, Download, DownloadState, DownloadStatus, Job, JobStatus, JobType,
	MetadataCacheEntry, QualityProfile, Request, RequestStatus, Role, UserSettings,
};
use lunu_core::repo::{
	DownloadRepo, JobRepo, MetadataCacheRepo, QualityProfileRepo, RequestRepo, SettingsRepo,
	UserRepo, UserSettingsRepo,
};
use lunu_core::services::{
	ApiKeyService, AuthService, InviteService, JobService, MetadataService, MonitorService,
	RequestService, SettingsService,
};
use lunu_core::traits::{DownloadClient, MetadataProvider};
use sqlx::any::{AnyPoolOptions, install_default_drivers};

use crate::repos::{
	SqlxApiKeyRepo, SqlxDownloadRepo, SqlxInviteRepo, SqlxJobRepo, SqlxMetadataCacheRepo,
	SqlxQualityProfileRepo, SqlxRequestRepo, SqlxSessionRepo, SqlxSettingsRepo, SqlxUserRepo,
	SqlxUserSettingsRepo,
};
use crate::{Db, run_migrations};

async fn memory_db() -> Db {
	install_default_drivers();
	let db = AnyPoolOptions::new()
		.max_connections(1)
		.connect("sqlite::memory:")
		.await
		.unwrap();
	run_migrations(&db).await.unwrap();
	db
}

fn auth_service(db: &Db) -> AuthService {
	AuthService::new(
		Arc::new(SqlxUserRepo::new(db.clone())),
		Arc::new(SqlxSessionRepo::new(db.clone())),
		Arc::new(SqlxInviteRepo::new(db.clone())),
	)
}

#[tokio::test]
async fn auth_setup_login_validate_logout() {
	let db = memory_db().await;
	let auth = auth_service(&db);

	assert!(auth.needs_setup().await.unwrap());
	let admin = auth
		.setup_first_admin("admin", "password123", None)
		.await
		.unwrap();
	assert_eq!(admin.user.role, Role::Admin);
	assert!(!auth.needs_setup().await.unwrap());
	assert!(auth.setup_first_admin("x", "y", None).await.is_err());

	assert!(auth.login("admin", "wrong").await.is_err());
	let authed = auth.login("admin", "password123").await.unwrap();
	assert_eq!(authed.user.username, "admin");

	let validated = auth
		.validate_session(&authed.session_token)
		.await
		.unwrap()
		.unwrap();
	assert_eq!(validated.id, admin.user.id);

	auth.logout(&authed.session_token).await.unwrap();
	assert!(
		auth.validate_session(&authed.session_token)
			.await
			.unwrap()
			.is_none()
	);
}

#[tokio::test]
async fn register_with_invite_then_exhausted() {
	let db = memory_db().await;
	let auth = auth_service(&db);
	let invites = InviteService::new(Arc::new(SqlxInviteRepo::new(db.clone())));

	let admin = auth
		.setup_first_admin("admin", "password123", None)
		.await
		.unwrap();
	let issued = invites
		.create(&admin.user.id, Role::User, None, 1, None)
		.await
		.unwrap();

	let registered = auth
		.register_with_invite(&issued.code, "bob", "hunter2password")
		.await
		.unwrap();
	assert_eq!(registered.user.username, "bob");
	assert_eq!(registered.user.role, Role::User);

	assert!(
		auth.register_with_invite(&issued.code, "carol", "password123")
			.await
			.is_err()
	);
}

#[tokio::test]
async fn settings_encrypts_secret_values() {
	let db = memory_db().await;
	let encryptor = Encryptor::new("dev-master-key-value", SETTINGS_ENCRYPTION_CONTEXT).unwrap();
	let settings = SettingsService::new(Arc::new(SqlxSettingsRepo::new(db.clone())), encryptor);

	settings
		.set("qbittorrent_password", "s3cret", true)
		.await
		.unwrap();
	settings.set("download_dir", "/data", false).await.unwrap();

	assert_eq!(
		settings
			.get("qbittorrent_password")
			.await
			.unwrap()
			.as_deref(),
		Some("s3cret")
	);
	assert_eq!(
		settings.get("download_dir").await.unwrap().as_deref(),
		Some("/data")
	);

	let stored = SqlxSettingsRepo::new(db.clone())
		.get("qbittorrent_password")
		.await
		.unwrap()
		.unwrap();
	assert!(stored.encrypted);
	assert_ne!(stored.value, "s3cret");
}

#[tokio::test]
async fn api_key_issue_verify_revoke() {
	let db = memory_db().await;
	let keys = ApiKeyService::new(Arc::new(SqlxApiKeyRepo::new(db.clone())));

	let issued = keys
		.issue("user-1", "cli", vec!["read".to_string()], None)
		.await
		.unwrap();

	let verified = keys.verify(&issued.secret).await.unwrap().unwrap();
	assert_eq!(verified.user_id, "user-1");

	assert!(
		keys.revoke_for_user("someone-else", &issued.api_key.id)
			.await
			.is_err()
	);

	keys.revoke_for_user("user-1", &issued.api_key.id)
		.await
		.unwrap();
	assert!(keys.verify(&issued.secret).await.unwrap().is_none());
}

#[tokio::test]
async fn metadata_cache_put_get_upsert() {
	let db = memory_db().await;
	let cache = SqlxMetadataCacheRepo::new(db.clone());

	assert!(
		cache
			.get("audnexus", "book", "B01")
			.await
			.unwrap()
			.is_none()
	);

	let entry = |payload: &str| MetadataCacheEntry {
		provider: "audnexus".to_string(),
		kind: "book".to_string(),
		key: "B01".to_string(),
		payload: payload.to_string(),
		fetched_at: Utc::now(),
	};

	cache.put(&entry("{\"v\":1}")).await.unwrap();
	assert_eq!(
		cache
			.get("audnexus", "book", "B01")
			.await
			.unwrap()
			.unwrap()
			.payload,
		"{\"v\":1}"
	);

	cache.put(&entry("{\"v\":2}")).await.unwrap();
	assert_eq!(
		cache
			.get("audnexus", "book", "B01")
			.await
			.unwrap()
			.unwrap()
			.payload,
		"{\"v\":2}"
	);
}

#[tokio::test]
async fn request_lifecycle_and_quota_count() {
	let db = memory_db().await;
	let requests = SqlxRequestRepo::new(db.clone());

	let now = Utc::now();
	let request = Request {
		id: "r1".to_string(),
		user_id: "u1".to_string(),
		asin: "B01".to_string(),
		title: "The Hobbit".to_string(),
		author: Some("Tolkien".to_string()),
		cover_url: None,
		status: RequestStatus::Pending,
		approved_by: None,
		created_at: now,
		updated_at: now,
	};
	requests.create(&request).await.unwrap();

	let found = requests
		.find_by_user_and_asin("u1", "B01")
		.await
		.unwrap()
		.unwrap();
	assert_eq!(found.status, RequestStatus::Pending);
	assert_eq!(found.author.as_deref(), Some("Tolkien"));

	let mut approved = found;
	approved.status = RequestStatus::Approved;
	approved.approved_by = Some("admin".to_string());
	requests.update(&approved).await.unwrap();
	assert_eq!(
		requests.find_by_id("r1").await.unwrap().unwrap().status,
		RequestStatus::Approved
	);

	assert_eq!(
		requests
			.count_for_user_since("u1", now - chrono::Duration::days(1))
			.await
			.unwrap(),
		1
	);
	assert_eq!(requests.list_for_user("u1").await.unwrap().len(), 1);
}

#[tokio::test]
async fn duplicate_username_is_conflict_not_db_error() {
	use lunu_core::Error;
	use lunu_core::models::{AuthSource, User};

	let db = memory_db().await;
	let repo = SqlxUserRepo::new(db.clone());

	let now = Utc::now();
	let user = |id: &str| User {
		id: id.to_string(),
		username: "alice".to_string(),
		email: None,
		password_hash: Some("hash".to_string()),
		role: Role::User,
		auth_source: AuthSource::Local,
		enabled: true,
		created_at: now,
		updated_at: now,
	};

	repo.create(&user("u1")).await.unwrap();
	match repo.create(&user("u2")).await {
		Err(Error::Conflict(_)) => {}
		other => panic!("expected Conflict on duplicate username, got {other:?}"),
	}
}

#[tokio::test]
async fn user_settings_upsert() {
	let db = memory_db().await;
	let settings = SqlxUserSettingsRepo::new(db.clone());

	assert!(settings.get("u1").await.unwrap().is_none());

	settings
		.upsert(&UserSettings {
			user_id: "u1".to_string(),
			auto_approve: true,
			request_quota: Some(5),
			quota_days: Some(7),
			updated_at: Utc::now(),
		})
		.await
		.unwrap();

	let loaded = settings.get("u1").await.unwrap().unwrap();
	assert!(loaded.auto_approve);
	assert_eq!(loaded.request_quota, Some(5));
	assert_eq!(loaded.quota_days, Some(7));
}

#[tokio::test]
async fn download_create_and_set_state() {
	let db = memory_db().await;
	let repo = SqlxDownloadRepo::new(db.clone());

	let now = Utc::now();
	let download = Download {
		id: "d1".to_string(),
		request_id: "r1".to_string(),
		client: "qbittorrent".to_string(),
		category: "lunu".to_string(),
		release_title: "The Hobbit [M4B]".to_string(),
		indexer: "MyTracker".to_string(),
		download_url: "magnet:?xt=urn:btih:abc".to_string(),
		info_hash: Some("abc".to_string()),
		state: DownloadState::Queued,
		progress: 0,
		created_at: now,
		updated_at: now,
	};
	repo.create(&download).await.unwrap();

	let found = repo.find_by_request("r1").await.unwrap().unwrap();
	assert_eq!(found.id, "d1");
	assert_eq!(found.state, DownloadState::Queued);
	assert_eq!(found.release_title, "The Hobbit [M4B]");
	assert_eq!(found.info_hash.as_deref(), Some("abc"));

	repo.update_status("d1", DownloadState::Downloading, 42, Utc::now())
		.await
		.unwrap();
	let updated = repo.find_by_id("d1").await.unwrap().unwrap();
	assert_eq!(updated.state, DownloadState::Downloading);
	assert_eq!(updated.progress, 42);
	assert_eq!(repo.list().await.unwrap().len(), 1);
}

struct StubProvider;

#[async_trait]
impl MetadataProvider for StubProvider {
	fn id(&self) -> &'static str {
		"stub"
	}
	async fn search(&self, _query: &str, _region: &str) -> CoreResult<Vec<Book>> {
		Ok(Vec::new())
	}
	async fn get_book(&self, _asin: &str, _region: &str) -> CoreResult<Option<Book>> {
		Ok(None)
	}
	async fn get_chapters(&self, _asin: &str, _region: &str) -> CoreResult<Option<Chapters>> {
		Ok(None)
	}
}

#[tokio::test]
async fn approving_a_request_enqueues_a_grab_job() {
	let db = memory_db().await;

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let requests = request_service(&db, jobs.clone());

	let now = Utc::now();
	let request = Request {
		id: "r1".to_string(),
		user_id: "u1".to_string(),
		asin: "B01".to_string(),
		title: "The Hobbit".to_string(),
		author: None,
		cover_url: None,
		status: RequestStatus::Pending,
		approved_by: None,
		created_at: now,
		updated_at: now,
	};
	SqlxRequestRepo::new(db.clone())
		.create(&request)
		.await
		.unwrap();

	let approved = requests.approve("admin", "r1").await.unwrap();
	assert_eq!(approved.status, RequestStatus::Approved);

	let listed = jobs.list().await.unwrap();
	assert_eq!(listed.len(), 1);
	assert_eq!(listed[0].job_type, JobType::Grab);
	assert!(listed[0].payload.contains("r1"));
}

struct FakeClient {
	response: Option<DownloadStatus>,
}

#[async_trait]
impl DownloadClient for FakeClient {
	fn id(&self) -> &'static str {
		"fake"
	}
	async fn add(&self, _download_url: &str, _category: &str) -> CoreResult<()> {
		Ok(())
	}
	async fn status(&self, _info_hash: &str) -> CoreResult<Option<DownloadStatus>> {
		Ok(self.response.clone())
	}
}

fn request_service(db: &Db, jobs: Arc<JobService>) -> Arc<RequestService> {
	let encryptor = Encryptor::new("dev-master-key-value", SETTINGS_ENCRYPTION_CONTEXT).unwrap();
	let settings = Arc::new(SettingsService::new(
		Arc::new(SqlxSettingsRepo::new(db.clone())),
		encryptor,
	));
	let metadata = Arc::new(MetadataService::new(
		Arc::new(StubProvider),
		Arc::new(SqlxMetadataCacheRepo::new(db.clone())),
		settings,
	));
	Arc::new(RequestService::new(
		Arc::new(SqlxRequestRepo::new(db.clone())),
		Arc::new(SqlxUserSettingsRepo::new(db.clone())),
		metadata,
		jobs,
	))
}

async fn seed_download(db: &Db, at: chrono::DateTime<Utc>) {
	SqlxRequestRepo::new(db.clone())
		.create(&Request {
			id: "r1".to_string(),
			user_id: "u1".to_string(),
			asin: "B01".to_string(),
			title: "The Hobbit".to_string(),
			author: None,
			cover_url: None,
			status: RequestStatus::Downloading,
			approved_by: Some("admin".to_string()),
			created_at: at,
			updated_at: at,
		})
		.await
		.unwrap();
	SqlxDownloadRepo::new(db.clone())
		.create(&Download {
			id: "d1".to_string(),
			request_id: "r1".to_string(),
			client: "qbittorrent".to_string(),
			category: "lunu".to_string(),
			release_title: "The Hobbit [M4B]".to_string(),
			indexer: "MyTracker".to_string(),
			download_url: "magnet:?xt=urn:btih:abc".to_string(),
			info_hash: Some("abc".to_string()),
			state: DownloadState::Downloading,
			progress: 10,
			created_at: at,
			updated_at: at,
		})
		.await
		.unwrap();
}

#[tokio::test]
async fn monitor_marks_request_importing_on_completion() {
	let db = memory_db().await;
	seed_download(&db, Utc::now()).await;

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let downloads = Arc::new(SqlxDownloadRepo::new(db.clone()));
	let client = Arc::new(FakeClient {
		response: Some(DownloadStatus {
			state: DownloadState::Completed,
			progress: 1.0,
			content_path: Some("/library/x".to_string()),
		}),
	});
	let monitor = MonitorService::new(
		downloads.clone(),
		client,
		request_service(&db, jobs.clone()),
		jobs.clone(),
	);

	monitor.poll("d1", 0).await.unwrap();

	assert_eq!(
		SqlxRequestRepo::new(db.clone())
			.find_by_id("r1")
			.await
			.unwrap()
			.unwrap()
			.status,
		RequestStatus::Importing
	);
	let download = downloads.find_by_id("d1").await.unwrap().unwrap();
	assert_eq!(download.state, DownloadState::Completed);
	assert_eq!(download.progress, 100);
}

#[tokio::test]
async fn monitor_reschedules_while_downloading() {
	let db = memory_db().await;
	seed_download(&db, Utc::now()).await;

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let downloads = Arc::new(SqlxDownloadRepo::new(db.clone()));
	let client = Arc::new(FakeClient {
		response: Some(DownloadStatus {
			state: DownloadState::Downloading,
			progress: 0.5,
			content_path: None,
		}),
	});
	let monitor = MonitorService::new(
		downloads.clone(),
		client,
		request_service(&db, jobs.clone()),
		jobs.clone(),
	);

	monitor.poll("d1", 0).await.unwrap();

	let listed = jobs.list().await.unwrap();
	assert_eq!(listed.len(), 1);
	assert_eq!(listed[0].job_type, JobType::MonitorDownload);
	assert_eq!(
		downloads.find_by_id("d1").await.unwrap().unwrap().progress,
		50
	);
}

#[tokio::test]
async fn monitor_fails_after_max_misses() {
	let db = memory_db().await;
	seed_download(&db, Utc::now()).await;

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let downloads = Arc::new(SqlxDownloadRepo::new(db.clone()));
	let client = Arc::new(FakeClient { response: None });
	let monitor = MonitorService::new(
		downloads.clone(),
		client,
		request_service(&db, jobs.clone()),
		jobs.clone(),
	);

	monitor.poll("d1", MONITOR_MAX_MISSES - 1).await.unwrap();

	assert_eq!(
		downloads.find_by_id("d1").await.unwrap().unwrap().state,
		DownloadState::Failed
	);
	assert_eq!(
		SqlxRequestRepo::new(db.clone())
			.find_by_id("r1")
			.await
			.unwrap()
			.unwrap()
			.status,
		RequestStatus::Failed
	);
	assert_eq!(jobs.list().await.unwrap().len(), 0);
}

fn pending_job(id: &str, at: chrono::DateTime<Utc>) -> Job {
	Job {
		id: id.to_string(),
		job_type: JobType::Grab,
		payload: "{\"request\":\"r1\"}".to_string(),
		status: JobStatus::Pending,
		attempts: 0,
		max_attempts: 3,
		run_after: at,
		locked_by: None,
		locked_at: None,
		last_error: None,
		created_at: at,
		updated_at: at,
	}
}

#[tokio::test]
async fn job_claim_is_atomic_and_lifecycle_transitions() {
	let db = memory_db().await;
	let repo = SqlxJobRepo::new(db.clone());

	let now = Utc::now();
	repo.create(&pending_job("j1", now)).await.unwrap();

	let claimed = repo
		.claim_next("worker-a", Utc::now())
		.await
		.unwrap()
		.unwrap();
	assert_eq!(claimed.id, "j1");
	assert_eq!(claimed.status, JobStatus::Running);
	assert_eq!(claimed.attempts, 1);
	assert_eq!(claimed.locked_by.as_deref(), Some("worker-a"));

	assert!(
		repo.claim_next("worker-b", Utc::now())
			.await
			.unwrap()
			.is_none()
	);

	let future = Utc::now() + chrono::Duration::seconds(30);
	repo.reschedule("j1", "temporary", future, Utc::now())
		.await
		.unwrap();
	let after = repo.find_by_id("j1").await.unwrap().unwrap();
	assert_eq!(after.status, JobStatus::Pending);
	assert_eq!(after.attempts, 1);
	assert_eq!(after.last_error.as_deref(), Some("temporary"));
	assert!(after.locked_by.is_none());

	assert!(
		repo.claim_next("worker-a", Utc::now())
			.await
			.unwrap()
			.is_none()
	);

	let reclaimed = repo
		.claim_next("worker-a", future + chrono::Duration::seconds(1))
		.await
		.unwrap()
		.unwrap();
	assert_eq!(reclaimed.attempts, 2);

	repo.complete("j1", Utc::now()).await.unwrap();
	assert_eq!(
		repo.find_by_id("j1").await.unwrap().unwrap().status,
		JobStatus::Completed
	);
	assert_eq!(repo.list().await.unwrap().len(), 1);
}

#[tokio::test]
async fn reap_stale_returns_running_jobs_to_pending() {
	let db = memory_db().await;
	let repo = SqlxJobRepo::new(db.clone());

	let now = Utc::now();
	repo.create(&pending_job("j2", now)).await.unwrap();
	repo.claim_next("worker-a", now).await.unwrap().unwrap();

	assert_eq!(
		repo.reap_stale(now - chrono::Duration::seconds(300), Utc::now())
			.await
			.unwrap(),
		0
	);

	let reaped = repo
		.reap_stale(now + chrono::Duration::seconds(1), Utc::now())
		.await
		.unwrap();
	assert_eq!(reaped, 1);

	let after = repo.find_by_id("j2").await.unwrap().unwrap();
	assert_eq!(after.status, JobStatus::Pending);
	assert!(after.locked_by.is_none());
	assert_eq!(after.attempts, 1);
}

#[tokio::test]
async fn quality_profile_crud_and_default() {
	let db = memory_db().await;
	let repo = SqlxQualityProfileRepo::new(db.clone());

	let now = Utc::now();
	let profile = QualityProfile {
		id: "p1".to_string(),
		name: "Audiobook".to_string(),
		allowed_formats: vec!["m4b".to_string(), "mp3".to_string()],
		preferred_formats: vec!["m4b".to_string()],
		min_seeders: 2,
		min_size_mb: Some(10),
		max_size_mb: None,
		seeder_weight: 1,
		format_weight: 100,
		is_default: true,
		created_at: now,
		updated_at: now,
	};
	repo.create(&profile).await.unwrap();

	let loaded = repo.find_by_id("p1").await.unwrap().unwrap();
	assert_eq!(loaded.allowed_formats, vec!["m4b", "mp3"]);
	assert_eq!(loaded.min_seeders, 2);
	assert_eq!(loaded.min_size_mb, Some(10));
	assert!(loaded.is_default);

	assert_eq!(repo.find_default().await.unwrap().unwrap().id, "p1");

	let mut second = profile.clone();
	second.id = "p2".to_string();
	second.is_default = false;
	repo.create(&second).await.unwrap();

	repo.set_default("p2").await.unwrap();
	assert_eq!(repo.find_default().await.unwrap().unwrap().id, "p2");
	assert!(!repo.find_by_id("p1").await.unwrap().unwrap().is_default);
	assert_eq!(repo.list().await.unwrap().len(), 2);
}
