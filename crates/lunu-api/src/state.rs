use std::sync::Arc;

use lunu_config::BootstrapConfig;
use lunu_core::Result;
use lunu_core::consts::auth::{
	AUTH_RATE_LIMIT_ATTEMPTS, AUTH_RATE_LIMIT_WINDOW_SECS, METADATA_RATE_LIMIT_ATTEMPTS,
	METADATA_RATE_LIMIT_WINDOW_SECS,
};
use lunu_core::consts::crypto::{MFA_ENCRYPTION_CONTEXT, SETTINGS_ENCRYPTION_CONTEXT};
use lunu_core::crypto::Encryptor;
use lunu_core::services::{
	ActivityService, ApiKeyService, AuthService, GrabService, ImportService, InviteService,
	IssueService, JobService, LibraryService, LogBuffer, MediaService, MergeService,
	MetadataService, MonitorService, NotificationInboxService, NotificationService,
	QualityProfileService, ReleaseService, RequestService, SchedulerService, SettingsService,
	UserService, WatchService, WorkService,
};
use lunu_core::traits::Mailer;
use lunu_db::Db;
use lunu_db::repos::{
	SqlxActivityRepo, SqlxApiKeyRepo, SqlxBlocklistRepo, SqlxDownloadRepo,
	SqlxEmailVerificationRepo, SqlxInviteRepo, SqlxIssueRepo, SqlxJobRepo, SqlxMediaRepo,
	SqlxMetadataCacheRepo, SqlxMfaRecoveryCodeRepo, SqlxPasswordResetRepo, SqlxQualityProfileRepo,
	SqlxRequestRepo, SqlxScheduleRepo, SqlxSessionRepo, SqlxSettingsRepo, SqlxUserMfaRepo,
	SqlxUserNotificationRepo, SqlxUserRepo, SqlxUserSettingsRepo, SqlxWatchRepo, SqlxWorkRepo,
};

use crate::hub::EventHub;
use crate::rate_limit::RateLimiter;
use crate::rosters;
use lunu_integrations::audio::FfmpegMerger;
use lunu_integrations::auth::{AudiobookshelfProvider, OidcClient};
use lunu_integrations::indexer::ProwlarrClient;
use lunu_integrations::library::{AbsLibrary, FileSidecarWriter, HardlinkImporter};
use lunu_integrations::notify::{EmailNotifier, NtfyChannel, SmtpMailer, WebhookChannel};

pub struct LogControl {
	setter: Box<dyn Fn(&str) -> bool + Send + Sync>,
	current: std::sync::RwLock<String>,
}

impl LogControl {
	pub fn new(initial: &str, setter: Box<dyn Fn(&str) -> bool + Send + Sync>) -> Self {
		Self {
			setter,
			current: std::sync::RwLock::new(initial.to_string()),
		}
	}

	pub fn set(&self, level: &str) -> bool {
		let applied = (self.setter)(level);
		if applied {
			*self.current.write().expect("log level lock") = level.to_string();
		}
		applied
	}

	pub fn current(&self) -> String {
		self.current.read().expect("log level lock").clone()
	}
}

pub struct AppState {
	pub db: Db,
	pub config: Arc<BootstrapConfig>,
	pub version: &'static str,
	pub logs: Arc<LogBuffer>,
	pub log_control: Arc<LogControl>,
	pub auth: Arc<AuthService>,
	pub users: Arc<UserService>,
	pub api_keys: Arc<ApiKeyService>,
	pub invites: Arc<InviteService>,
	pub settings: Arc<SettingsService>,
	pub metadata: Arc<MetadataService>,
	pub requests: Arc<RequestService>,
	pub watches: Arc<WatchService>,
	pub works: Arc<WorkService>,
	pub releases: Arc<ReleaseService>,
	pub quality_profiles: Arc<QualityProfileService>,
	pub grabs: Arc<GrabService>,
	pub jobs: Arc<JobService>,
	pub scheduler: Arc<SchedulerService>,
	pub monitor: Arc<MonitorService>,
	pub imports: Arc<ImportService>,
	pub merges: Arc<MergeService>,
	pub activity: Arc<ActivityService>,
	pub media: Arc<MediaService>,
	pub library: Arc<LibraryService>,
	pub issues: Arc<IssueService>,
	pub inbox: Arc<NotificationInboxService>,
	pub notifications: Arc<NotificationService>,
	pub hub: Arc<EventHub>,
	pub auth_rate_limiter: Arc<RateLimiter>,
	pub metadata_rate_limiter: Arc<RateLimiter>,
}

impl AppState {
	pub fn build(
		db: Db,
		config: BootstrapConfig,
		version: &'static str,
		logs: Arc<LogBuffer>,
		log_control: Arc<LogControl>,
	) -> Result<Self> {
		let users_repo = Arc::new(SqlxUserRepo::new(db.clone()));
		let sessions_repo = Arc::new(SqlxSessionRepo::new(db.clone()));
		let api_keys_repo = Arc::new(SqlxApiKeyRepo::new(db.clone()));
		let invites_repo = Arc::new(SqlxInviteRepo::new(db.clone()));
		let settings_repo = Arc::new(SqlxSettingsRepo::new(db.clone()));
		let metadata_cache_repo = Arc::new(SqlxMetadataCacheRepo::new(db.clone()));
		let requests_repo = Arc::new(SqlxRequestRepo::new(db.clone()));
		let user_settings_repo = Arc::new(SqlxUserSettingsRepo::new(db.clone()));
		let quality_profiles_repo = Arc::new(SqlxQualityProfileRepo::new(db.clone()));
		let downloads_repo = Arc::new(SqlxDownloadRepo::new(db.clone()));
		let jobs_repo = Arc::new(SqlxJobRepo::new(db.clone()));
		let activity_repo = Arc::new(SqlxActivityRepo::new(db.clone()));
		let blocklist_repo = Arc::new(SqlxBlocklistRepo::new(db.clone()));
		let media_repo = Arc::new(SqlxMediaRepo::new(db.clone()));

		let encryptor = Encryptor::new(&config.master_key, SETTINGS_ENCRYPTION_CONTEXT)?;
		let settings = Arc::new(SettingsService::new(settings_repo, encryptor));
		let mailer: Arc<dyn Mailer> = Arc::new(SmtpMailer::new(settings.clone()));

		let auth = Arc::new(
			AuthService::new(
				users_repo.clone(),
				sessions_repo.clone(),
				invites_repo.clone(),
				Some(Arc::new(AudiobookshelfProvider::new(settings.clone()))),
				Arc::new(SqlxPasswordResetRepo::new(db.clone())),
				Arc::new(SqlxEmailVerificationRepo::new(db.clone())),
				Arc::new(SqlxUserMfaRepo::new(db.clone())),
				Arc::new(SqlxMfaRecoveryCodeRepo::new(db.clone())),
				Encryptor::new(&config.master_key, MFA_ENCRYPTION_CONTEXT)?,
				settings.clone(),
				mailer.clone(),
			)
			.with_oidc(Arc::new(OidcClient::new(settings.clone()))),
		);
		let users = Arc::new(UserService::new(
			users_repo.clone(),
			sessions_repo,
			user_settings_repo.clone(),
		));
		let api_keys = Arc::new(ApiKeyService::new(api_keys_repo));
		let invites = Arc::new(InviteService::new(invites_repo));

		let metadata = Arc::new(MetadataService::new(
			rosters::metadata_providers(&settings),
			metadata_cache_repo,
			settings.clone(),
		));
		let jobs = Arc::new(JobService::new(jobs_repo));
		let scheduler = Arc::new(SchedulerService::new(
			Arc::new(SqlxScheduleRepo::new(db.clone())),
			jobs.clone(),
		));
		let hub = Arc::new(EventHub::new());
		let auth_rate_limiter = Arc::new(RateLimiter::new(
			AUTH_RATE_LIMIT_ATTEMPTS,
			std::time::Duration::from_secs(AUTH_RATE_LIMIT_WINDOW_SECS),
		));
		let metadata_rate_limiter = Arc::new(RateLimiter::new(
			METADATA_RATE_LIMIT_ATTEMPTS,
			std::time::Duration::from_secs(METADATA_RATE_LIMIT_WINDOW_SECS),
		));
		let activity = Arc::new(ActivityService::new(activity_repo, hub.clone()));
		let media = Arc::new(MediaService::new(media_repo.clone()));
		let works = Arc::new(WorkService::new(Arc::new(SqlxWorkRepo::new(db.clone()))));
		let library = Arc::new(LibraryService::new(
			Arc::new(AbsLibrary::new(settings.clone())),
			media_repo.clone(),
			metadata.clone(),
			works.clone(),
		));
		let inbox = Arc::new(NotificationInboxService::new(
			Arc::new(SqlxUserNotificationRepo::new(db.clone())),
			users_repo.clone(),
			hub.clone(),
		));
		let requests = Arc::new(RequestService::new(
			requests_repo.clone(),
			user_settings_repo,
			metadata.clone(),
			jobs.clone(),
			activity.clone(),
			downloads_repo.clone(),
			media_repo.clone(),
			inbox.clone(),
			works.clone(),
		));

		let watches = Arc::new(WatchService::new(
			Arc::new(SqlxWatchRepo::new(db.clone())),
			metadata.clone(),
			works.clone(),
			requests.clone(),
		));

		let indexer = Arc::new(ProwlarrClient::new(settings.clone()));
		let releases = Arc::new(ReleaseService::new(
			indexer,
			quality_profiles_repo.clone(),
			requests_repo.clone(),
			blocklist_repo,
		));
		let quality_profiles = Arc::new(QualityProfileService::new(quality_profiles_repo));

		let download_clients = rosters::download_clients(&settings);
		let monitor = Arc::new(MonitorService::new(
			downloads_repo.clone(),
			download_clients.clone(),
			requests.clone(),
			jobs.clone(),
			hub.clone(),
			settings.clone(),
		));
		let grabs = Arc::new(GrabService::new(
			downloads_repo.clone(),
			requests.clone(),
			releases.clone(),
			download_clients,
			jobs.clone(),
		));
		let issues = Arc::new(IssueService::new(
			Arc::new(SqlxIssueRepo::new(db.clone())),
			requests.clone(),
		));

		let merges = Arc::new(MergeService::new(
			media_repo.clone(),
			settings.clone(),
			Arc::new(FfmpegMerger::new(settings.clone())),
			jobs.clone(),
			activity.clone(),
			hub.clone(),
		));

		let importer = Arc::new(HardlinkImporter::new());
		let imports = Arc::new(ImportService::new(
			downloads_repo,
			requests.clone(),
			settings.clone(),
			importer,
			media.clone(),
			merges.clone(),
			Arc::new(FileSidecarWriter::new()),
		));

		let notifications = Arc::new(NotificationService::new(vec![
			Arc::new(WebhookChannel::generic(settings.clone())),
			Arc::new(WebhookChannel::discord(settings.clone())),
			Arc::new(WebhookChannel::slack(settings.clone())),
			Arc::new(WebhookChannel::apprise(settings.clone())),
			Arc::new(NtfyChannel::new(settings.clone())),
			Arc::new(EmailNotifier::new(
				mailer.clone(),
				users_repo,
				settings.clone(),
			)),
		]));

		Ok(Self {
			db,
			config: Arc::new(config),
			logs,
			log_control,
			version,
			auth,
			users,
			api_keys,
			invites,
			settings,
			metadata,
			requests,
			watches,
			works,
			releases,
			quality_profiles,
			grabs,
			jobs,
			monitor,
			imports,
			merges,
			activity,
			media,
			library,
			scheduler,
			issues,
			inbox,
			notifications,
			hub,
			auth_rate_limiter,
			metadata_rate_limiter,
		})
	}
}
