pub const METADATA_CACHE_TTL_DAYS: i64 = 14;

pub const AUDNEXUS_PROVIDER: &str = "audnexus";

pub const METADATA_AUDNEXUS_ENABLED: &str = "metadata_audnexus_enabled";
pub const METADATA_AUDNEXUS_PRIORITY: &str = "metadata_audnexus_priority";

pub const MIN_PROVIDER_PRIORITY: i64 = 1;
pub const MAX_PROVIDER_PRIORITY: i64 = 100;
pub const DEFAULT_PROVIDER_PRIORITY: i64 = 50;
pub const DEFAULT_PROVIDER_PRIORITY_VALUE: &str = "50";

pub struct ProviderSettings {
	pub provider: &'static str,
	pub enabled: &'static str,
	pub priority: &'static str,
}

pub const METADATA_PROVIDER_SETTINGS: &[ProviderSettings] = &[ProviderSettings {
	provider: AUDNEXUS_PROVIDER,
	enabled: METADATA_AUDNEXUS_ENABLED,
	priority: METADATA_AUDNEXUS_PRIORITY,
}];

pub fn provider_settings(provider_id: &str) -> Option<&'static ProviderSettings> {
	METADATA_PROVIDER_SETTINGS
		.iter()
		.find(|settings| settings.provider == provider_id)
}

pub const METADATA_REGION_SETTING: &str = "metadata_region";
pub const DEFAULT_METADATA_REGION: &str = "us";
pub const VALID_METADATA_REGIONS: &[&str] =
	&["au", "ca", "de", "es", "fr", "in", "it", "jp", "uk", "us"];
