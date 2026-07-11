use std::sync::Arc;

use lunu_config::BootstrapConfig;
use lunu_core::Result;
use lunu_core::consts::crypto::SETTINGS_ENCRYPTION_CONTEXT;
use lunu_core::crypto::Encryptor;
use lunu_core::services::{
	ApiKeyService, AuthService, InviteService, MetadataService, QualityProfileService,
	ReleaseService, RequestService, SettingsService, UserService,
};
use lunu_db::Db;
use lunu_db::repos::{
	SqlxApiKeyRepo, SqlxInviteRepo, SqlxMetadataCacheRepo, SqlxQualityProfileRepo, SqlxRequestRepo,
	SqlxSessionRepo, SqlxSettingsRepo, SqlxUserRepo, SqlxUserSettingsRepo,
};
use lunu_integrations::indexer::ProwlarrClient;
use lunu_integrations::metadata::AudnexusProvider;

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

		let encryptor = Encryptor::new(&config.master_key, SETTINGS_ENCRYPTION_CONTEXT)?;

		let auth = Arc::new(AuthService::new(
			users_repo.clone(),
			sessions_repo.clone(),
			invites_repo.clone(),
		));
		let users = Arc::new(UserService::new(
			users_repo,
			sessions_repo,
			user_settings_repo.clone(),
		));
		let api_keys = Arc::new(ApiKeyService::new(api_keys_repo));
		let invites = Arc::new(InviteService::new(invites_repo));
		let settings = Arc::new(SettingsService::new(settings_repo, encryptor));

		let provider = Arc::new(AudnexusProvider::with_default_region());
		let metadata = Arc::new(MetadataService::new(provider, metadata_cache_repo));
		let requests = Arc::new(RequestService::new(
			requests_repo.clone(),
			user_settings_repo,
			metadata.clone(),
		));

		let indexer = Arc::new(ProwlarrClient::new(settings.clone()));
		let releases = Arc::new(ReleaseService::new(
			indexer,
			quality_profiles_repo.clone(),
			requests_repo,
		));
		let quality_profiles = Arc::new(QualityProfileService::new(quality_profiles_repo));

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
		})
	}
}
