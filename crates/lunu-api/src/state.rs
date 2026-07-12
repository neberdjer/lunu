use std::sync::Arc;

use lunu_config::BootstrapConfig;
use lunu_core::Result;
use lunu_core::consts::auth::{AUTH_RATE_LIMIT_ATTEMPTS, AUTH_RATE_LIMIT_WINDOW_SECS};
use lunu_core::consts::crypto::SETTINGS_ENCRYPTION_CONTEXT;
use lunu_core::crypto::Encryptor;
use lunu_core::services::{
	ActivityService, ApiKeyService, AuthService, GrabService, ImportService, InviteService,
	IssueService, JobService, MediaService, MetadataService, MonitorService,
	NotificationInboxService, NotificationService, QualityProfileService, ReleaseService,
	RequestService, SettingsService, UserService,
};
use lunu_core::traits::Mailer;
use lunu_db::Db;
use lunu_db::repos::{
	SqlxActivityRepo, SqlxApiKeyRepo, SqlxBlocklistRepo, SqlxDownloadRepo, SqlxInviteRepo,
	SqlxIssueRepo, SqlxJobRepo, SqlxMediaRepo, SqlxMetadataCacheRepo, SqlxPasswordResetRepo,
	SqlxQualityProfileRepo, SqlxRequestRepo, SqlxSessionRepo, SqlxSettingsRepo,
	SqlxUserNotificationRepo, SqlxUserRepo, SqlxUserSettingsRepo,
};

use crate::hub::EventHub;
use crate::rate_limit::RateLimiter;
use lunu_integrations::auth::AudiobookshelfProvider;
use lunu_integrations::download::QbittorrentClient;
use lunu_integrations::indexer::ProwlarrClient;
use lunu_integrations::library::HardlinkImporter;
use lunu_integrations::metadata::AudnexusProvider;
use lunu_integrations::notify::{EmailNotifier, SmtpMailer, WebhookChannel};

pub struct AppState {
	pub db: Db,
	pub config: Arc<BootstrapConfig>,
	pub version: &'static str,
	pub auth: Arc<AuthService>,
	pub users: Arc<UserService>,
	pub api_keys: Arc<ApiKeyService>,
	pub invites: Arc<InviteService>,
	pub settings: Arc<SettingsService>,
	pub metadata: Arc<MetadataService>,
	pub requests: Arc<RequestService>,
	pub releases: Arc<ReleaseService>,
	pub quality_profiles: Arc<QualityProfileService>,
	pub grabs: Arc<GrabService>,
	pub jobs: Arc<JobService>,
	pub monitor: Arc<MonitorService>,
	pub imports: Arc<ImportService>,
	pub activity: Arc<ActivityService>,
	pub media: Arc<MediaService>,
	pub issues: Arc<IssueService>,
	pub inbox: Arc<NotificationInboxService>,
	pub notifications: Arc<NotificationService>,
	pub hub: Arc<EventHub>,
	pub auth_rate_limiter: Arc<RateLimiter>,
}

impl AppState {
	pub fn build(db: Db, config: BootstrapConfig, version: &'static str) -> Result<Self> {
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

		let auth = Arc::new(AuthService::new(
			users_repo.clone(),
			sessions_repo.clone(),
			invites_repo.clone(),
			Some(Arc::new(AudiobookshelfProvider::new(settings.clone()))),
			Arc::new(SqlxPasswordResetRepo::new(db.clone())),
			mailer.clone(),
		));
		let users = Arc::new(UserService::new(
			users_repo.clone(),
			sessions_repo,
			user_settings_repo.clone(),
		));
		let api_keys = Arc::new(ApiKeyService::new(api_keys_repo));
		let invites = Arc::new(InviteService::new(invites_repo));

		let provider = Arc::new(AudnexusProvider::new());
		let metadata = Arc::new(MetadataService::new(
			provider,
			metadata_cache_repo,
			settings.clone(),
		));
		let jobs = Arc::new(JobService::new(jobs_repo));
		let hub = Arc::new(EventHub::new());
		let auth_rate_limiter = Arc::new(RateLimiter::new(
			AUTH_RATE_LIMIT_ATTEMPTS,
			std::time::Duration::from_secs(AUTH_RATE_LIMIT_WINDOW_SECS),
		));
		let activity = Arc::new(ActivityService::new(activity_repo, hub.clone()));
		let media = Arc::new(MediaService::new(media_repo.clone()));
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
		));

		let indexer = Arc::new(ProwlarrClient::new(settings.clone()));
		let releases = Arc::new(ReleaseService::new(
			indexer,
			quality_profiles_repo.clone(),
			requests_repo.clone(),
			blocklist_repo,
		));
		let quality_profiles = Arc::new(QualityProfileService::new(quality_profiles_repo));

		let download_client = Arc::new(QbittorrentClient::new(settings.clone()));
		let monitor = Arc::new(MonitorService::new(
			downloads_repo.clone(),
			download_client.clone(),
			requests.clone(),
			jobs.clone(),
			hub.clone(),
		));
		let grabs = Arc::new(GrabService::new(
			downloads_repo.clone(),
			requests.clone(),
			releases.clone(),
			download_client,
			jobs.clone(),
		));
		let issues = Arc::new(IssueService::new(
			Arc::new(SqlxIssueRepo::new(db.clone())),
			requests.clone(),
		));

		let importer = Arc::new(HardlinkImporter::new());
		let imports = Arc::new(ImportService::new(
			downloads_repo,
			requests.clone(),
			settings.clone(),
			importer,
			media.clone(),
		));

		let notifications = Arc::new(NotificationService::new(vec![
			Arc::new(WebhookChannel::generic(settings.clone())),
			Arc::new(WebhookChannel::discord(settings.clone())),
			Arc::new(WebhookChannel::slack(settings.clone())),
			Arc::new(EmailNotifier::new(
				mailer.clone(),
				users_repo,
				settings.clone(),
			)),
		]));

		Ok(Self {
			db,
			config: Arc::new(config),
			version,
			auth,
			users,
			api_keys,
			invites,
			settings,
			metadata,
			requests,
			releases,
			quality_profiles,
			grabs,
			jobs,
			monitor,
			imports,
			activity,
			media,
			issues,
			inbox,
			notifications,
			hub,
			auth_rate_limiter,
		})
	}
}
