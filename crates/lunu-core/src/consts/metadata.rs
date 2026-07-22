pub const METADATA_CACHE_TTL_DAYS: i64 = 14;
pub const METADATA_GENRE_LIMIT: usize = 10;

pub const AUDNEXUS_PROVIDER: &str = "audnexus";
pub const OPENLIBRARY_PROVIDER: &str = "openlibrary";
pub const GOOGLE_BOOKS_PROVIDER: &str = "google_books";
pub const HARDCOVER_PROVIDER: &str = "hardcover";

pub const METADATA_AUDNEXUS_URL: &str = "metadata_audnexus_url";
pub const DEFAULT_AUDNEXUS_URL: &str = "https://api.audnex.us";
pub const METADATA_AUDNEXUS_ENABLED: &str = "metadata_audnexus_enabled";
pub const METADATA_AUDNEXUS_PRIORITY: &str = "metadata_audnexus_priority";
pub const METADATA_OPENLIBRARY_ENABLED: &str = "metadata_openlibrary_enabled";
pub const METADATA_OPENLIBRARY_PRIORITY: &str = "metadata_openlibrary_priority";
pub const METADATA_GOOGLE_BOOKS_ENABLED: &str = "metadata_google_books_enabled";
pub const METADATA_GOOGLE_BOOKS_PRIORITY: &str = "metadata_google_books_priority";
pub const METADATA_GOOGLE_BOOKS_API_KEY: &str = "metadata_google_books_api_key";
pub const METADATA_HARDCOVER_ENABLED: &str = "metadata_hardcover_enabled";
pub const METADATA_HARDCOVER_PRIORITY: &str = "metadata_hardcover_priority";
pub const METADATA_HARDCOVER_API_KEY: &str = "metadata_hardcover_api_key";

pub const MIN_PROVIDER_PRIORITY: i64 = 1;
pub const MAX_PROVIDER_PRIORITY: i64 = 100;
pub const DEFAULT_PROVIDER_PRIORITY: i64 = 50;
pub const DEFAULT_PROVIDER_PRIORITY_VALUE: &str = "50";

pub struct ProviderSettings {
	pub provider: &'static str,
	pub enabled: &'static str,
	pub priority: &'static str,
}

pub const METADATA_PROVIDER_SETTINGS: &[ProviderSettings] = &[
	ProviderSettings {
		provider: AUDNEXUS_PROVIDER,
		enabled: METADATA_AUDNEXUS_ENABLED,
		priority: METADATA_AUDNEXUS_PRIORITY,
	},
	ProviderSettings {
		provider: OPENLIBRARY_PROVIDER,
		enabled: METADATA_OPENLIBRARY_ENABLED,
		priority: METADATA_OPENLIBRARY_PRIORITY,
	},
	ProviderSettings {
		provider: GOOGLE_BOOKS_PROVIDER,
		enabled: METADATA_GOOGLE_BOOKS_ENABLED,
		priority: METADATA_GOOGLE_BOOKS_PRIORITY,
	},
	ProviderSettings {
		provider: HARDCOVER_PROVIDER,
		enabled: METADATA_HARDCOVER_ENABLED,
		priority: METADATA_HARDCOVER_PRIORITY,
	},
];

pub fn provider_settings(provider_id: &str) -> Option<&'static ProviderSettings> {
	METADATA_PROVIDER_SETTINGS
		.iter()
		.find(|settings| settings.provider == provider_id)
}

pub const METADATA_REGION_SETTING: &str = "metadata_region";
pub const DEFAULT_METADATA_REGION: &str = "us";
pub const VALID_METADATA_REGIONS: &[&str] =
	&["au", "ca", "de", "es", "fr", "in", "it", "jp", "uk", "us"];
