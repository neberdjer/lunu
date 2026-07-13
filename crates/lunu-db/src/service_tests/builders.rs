use super::*;

pub(super) struct NoopMailer;

#[async_trait]
impl Mailer for NoopMailer {
	async fn send(&self, _to: &str, _subject: &str, _html: &str) -> CoreResult<()> {
		Ok(())
	}
}

#[derive(Default)]
pub(super) struct RecordingMailer {
	sent: std::sync::atomic::AtomicUsize,
}

impl RecordingMailer {
	pub(super) fn count(&self) -> usize {
		self.sent.load(std::sync::atomic::Ordering::Relaxed)
	}
}

#[async_trait]
impl Mailer for RecordingMailer {
	async fn send(&self, _to: &str, _subject: &str, _html: &str) -> CoreResult<()> {
		self.sent.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
		Ok(())
	}
}

pub(super) fn auth_service(db: &Db) -> AuthService {
	auth_service_impl(db, None, Arc::new(NoopMailer))
}

pub(super) fn auth_service_with_provider(db: &Db, provider: Arc<dyn AuthProvider>) -> AuthService {
	auth_service_impl(db, Some(provider), Arc::new(NoopMailer))
}

pub(super) fn auth_service_with_mailer(db: &Db, mailer: Arc<dyn Mailer>) -> AuthService {
	auth_service_impl(db, None, mailer)
}

pub(super) fn auth_service_impl(
	db: &Db,
	provider: Option<Arc<dyn AuthProvider>>,
	mailer: Arc<dyn Mailer>,
) -> AuthService {
	AuthService::new(
		Arc::new(SqlxUserRepo::new(db.clone())),
		Arc::new(SqlxSessionRepo::new(db.clone())),
		Arc::new(SqlxInviteRepo::new(db.clone())),
		provider,
		Arc::new(SqlxPasswordResetRepo::new(db.clone())),
		mailer,
	)
}

pub(super) async fn seed_reset_code(db: &Db, user_id: &str, code: &str) {
	SqlxPasswordResetRepo::new(db.clone())
		.create(&PasswordResetToken {
			id: "t1".to_string(),
			user_id: user_id.to_string(),
			code_hash: hash_token(code),
			attempts: 0,
			created_at: Utc::now(),
			expires_at: Utc::now() + chrono::Duration::minutes(15),
		})
		.await
		.unwrap();
}

pub(super) async fn reset_token_count(db: &Db) -> i64 {
	sqlx::query_scalar("SELECT COUNT(*) FROM password_reset_tokens")
		.fetch_one(db)
		.await
		.unwrap()
}

pub(super) fn user_service(db: &Db) -> UserService {
	UserService::new(
		Arc::new(SqlxUserRepo::new(db.clone())),
		Arc::new(SqlxSessionRepo::new(db.clone())),
		Arc::new(SqlxUserSettingsRepo::new(db.clone())),
	)
}

pub(super) fn caller(id: &str, role: Role) -> User {
	let now = Utc::now();
	User {
		id: id.to_string(),
		username: id.to_string(),
		email: None,
		password_hash: None,
		role,
		auth_source: AuthSource::Local,
		display_name: None,
		locale: None,
		enabled: true,
		created_at: now,
		updated_at: now,
	}
}

pub(super) fn release(download_url: &str) -> Release {
	Release {
		title: "Book m4b".to_string(),
		indexer: "trk".to_string(),
		protocol: Protocol::Torrent,
		size: 500 * 1024 * 1024,
		seeders: 10,
		leechers: 0,
		download_url: download_url.to_string(),
		info_hash: None,
		info_url: None,
		publish_date: None,
	}
}

pub(super) struct StubProvider;

#[async_trait]
impl MetadataProvider for StubProvider {
	fn id(&self) -> &'static str {
		"stub"
	}
	async fn search(&self, _query: &str, _region: &str, _page: i64) -> CoreResult<Vec<Book>> {
		Ok(Vec::new())
	}
	async fn get_book(&self, _asin: &str, _region: &str) -> CoreResult<Option<Book>> {
		Ok(None)
	}
	async fn get_chapters(&self, _asin: &str, _region: &str) -> CoreResult<Option<Chapters>> {
		Ok(None)
	}
	async fn similar(&self, _asin: &str, _region: &str) -> CoreResult<Vec<Book>> {
		Ok(Vec::new())
	}
	async fn books_by_author(&self, _author_asin: &str, _region: &str) -> CoreResult<Vec<Book>> {
		Ok(Vec::new())
	}
}

pub(super) fn settings_service(db: &Db) -> Arc<SettingsService> {
	let encryptor = Encryptor::new("dev-master-key-value", SETTINGS_ENCRYPTION_CONTEXT).unwrap();
	Arc::new(SettingsService::new(
		Arc::new(SqlxSettingsRepo::new(db.clone())),
		encryptor,
	))
}

pub(super) fn request_service(db: &Db, jobs: Arc<JobService>) -> Arc<RequestService> {
	request_service_with_activity(db, jobs, activity_service(db))
}

pub(super) struct NoopPublisher;

impl EventPublisher for NoopPublisher {
	fn publish(&self, _event: &lunu_core::models::LiveEvent) {}
}

pub(super) fn activity_service(db: &Db) -> Arc<ActivityService> {
	Arc::new(ActivityService::new(
		Arc::new(SqlxActivityRepo::new(db.clone())),
		Arc::new(NoopPublisher),
	))
}

pub(super) fn request_service_with_activity(
	db: &Db,
	jobs: Arc<JobService>,
	activity: Arc<ActivityService>,
) -> Arc<RequestService> {
	let metadata = Arc::new(MetadataService::new(
		Arc::new(StubProvider),
		Arc::new(SqlxMetadataCacheRepo::new(db.clone())),
		settings_service(db),
	));
	Arc::new(RequestService::new(
		Arc::new(SqlxRequestRepo::new(db.clone())),
		Arc::new(SqlxUserSettingsRepo::new(db.clone())),
		metadata,
		jobs,
		activity,
		Arc::new(SqlxDownloadRepo::new(db.clone())),
		Arc::new(SqlxMediaRepo::new(db.clone())),
		Arc::new(NotificationInboxService::new(
			Arc::new(SqlxUserNotificationRepo::new(db.clone())),
			Arc::new(SqlxUserRepo::new(db.clone())),
			Arc::new(NoopPublisher),
		)),
	))
}

pub(super) async fn seed_download(db: &Db, at: chrono::DateTime<Utc>) {
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
			notes: None,
			quality_profile_id: None,
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

pub(super) fn pending_job(id: &str, at: chrono::DateTime<Utc>) -> Job {
	Job {
		id: id.to_string(),
		job_type: JobType::Grab,
		request_id: None,
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
