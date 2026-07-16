use super::{
	ABS_API_TOKEN, ABS_LIBRARY_ID, ABS_URL, BASE_URL, DEFAULT_PROVIDER_TOGGLE,
	DEFAULT_SMTP_ENCRYPTION, DEFAULT_TOGGLE, DISCORD_WEBHOOK_URL, DOWNLOAD_DIR,
	NOTIFICATION_WEBHOOK_URL, PROWLARR_API_KEY, PROWLARR_URL, QBITTORRENT_API_KEY,
	QBITTORRENT_PASSWORD, QBITTORRENT_URL, QBITTORRENT_USERNAME, REQUIRE_EMAIL_VERIFICATION,
	SLACK_WEBHOOK_URL, SMTP_ENCRYPTION, SMTP_ENCRYPTION_MODES, SMTP_FROM, SMTP_HOST, SMTP_PASSWORD,
	SMTP_PORT, SMTP_USERNAME, SettingKind, SettingSpec, TOGGLE_MODES,
};
use crate::consts::library::SETTING_LIBRARY_DIR;
use crate::consts::metadata::{
	DEFAULT_METADATA_REGION, DEFAULT_PROVIDER_PRIORITY_VALUE, MAX_PROVIDER_PRIORITY,
	METADATA_AUDNEXUS_ENABLED, METADATA_AUDNEXUS_PRIORITY, METADATA_OPENLIBRARY_ENABLED,
	METADATA_OPENLIBRARY_PRIORITY, METADATA_REGION_SETTING, MIN_PROVIDER_PRIORITY,
	VALID_METADATA_REGIONS,
};

pub const REGISTRY: &[SettingSpec] = &[
	SettingSpec {
		key: METADATA_AUDNEXUS_ENABLED,
		kind: SettingKind::Enum(TOGGLE_MODES),
		secret: false,
		default: Some(DEFAULT_PROVIDER_TOGGLE),
	},
	SettingSpec {
		key: METADATA_AUDNEXUS_PRIORITY,
		kind: SettingKind::Number {
			min: MIN_PROVIDER_PRIORITY,
			max: MAX_PROVIDER_PRIORITY,
		},
		secret: false,
		default: Some(DEFAULT_PROVIDER_PRIORITY_VALUE),
	},
	SettingSpec {
		key: METADATA_OPENLIBRARY_ENABLED,
		kind: SettingKind::Enum(TOGGLE_MODES),
		secret: false,
		default: Some(DEFAULT_PROVIDER_TOGGLE),
	},
	SettingSpec {
		key: METADATA_OPENLIBRARY_PRIORITY,
		kind: SettingKind::Number {
			min: MIN_PROVIDER_PRIORITY,
			max: MAX_PROVIDER_PRIORITY,
		},
		secret: false,
		default: Some(DEFAULT_PROVIDER_PRIORITY_VALUE),
	},
	SettingSpec {
		key: PROWLARR_URL,
		kind: SettingKind::Url,
		secret: false,
		default: None,
	},
	SettingSpec {
		key: PROWLARR_API_KEY,
		kind: SettingKind::Text,
		secret: true,
		default: None,
	},
	SettingSpec {
		key: QBITTORRENT_URL,
		kind: SettingKind::Url,
		secret: false,
		default: None,
	},
	SettingSpec {
		key: QBITTORRENT_API_KEY,
		kind: SettingKind::Text,
		secret: true,
		default: None,
	},
	SettingSpec {
		key: QBITTORRENT_USERNAME,
		kind: SettingKind::Text,
		secret: false,
		default: None,
	},
	SettingSpec {
		key: QBITTORRENT_PASSWORD,
		kind: SettingKind::Text,
		secret: true,
		default: None,
	},
	SettingSpec {
		key: DOWNLOAD_DIR,
		kind: SettingKind::Path,
		secret: false,
		default: None,
	},
	SettingSpec {
		key: SETTING_LIBRARY_DIR,
		kind: SettingKind::Path,
		secret: false,
		default: None,
	},
	SettingSpec {
		key: METADATA_REGION_SETTING,
		kind: SettingKind::Enum(VALID_METADATA_REGIONS),
		secret: false,
		default: Some(DEFAULT_METADATA_REGION),
	},
	SettingSpec {
		key: ABS_URL,
		kind: SettingKind::Url,
		secret: false,
		default: None,
	},
	SettingSpec {
		key: ABS_API_TOKEN,
		kind: SettingKind::Text,
		secret: true,
		default: None,
	},
	SettingSpec {
		key: ABS_LIBRARY_ID,
		kind: SettingKind::Text,
		secret: false,
		default: None,
	},
	SettingSpec {
		key: NOTIFICATION_WEBHOOK_URL,
		kind: SettingKind::Url,
		secret: false,
		default: None,
	},
	SettingSpec {
		key: DISCORD_WEBHOOK_URL,
		kind: SettingKind::Url,
		secret: true,
		default: None,
	},
	SettingSpec {
		key: SLACK_WEBHOOK_URL,
		kind: SettingKind::Url,
		secret: true,
		default: None,
	},
	SettingSpec {
		key: BASE_URL,
		kind: SettingKind::Url,
		secret: false,
		default: None,
	},
	SettingSpec {
		key: SMTP_HOST,
		kind: SettingKind::Text,
		secret: false,
		default: None,
	},
	SettingSpec {
		key: SMTP_PORT,
		kind: SettingKind::Text,
		secret: false,
		default: None,
	},
	SettingSpec {
		key: SMTP_USERNAME,
		kind: SettingKind::Text,
		secret: false,
		default: None,
	},
	SettingSpec {
		key: SMTP_PASSWORD,
		kind: SettingKind::Text,
		secret: true,
		default: None,
	},
	SettingSpec {
		key: SMTP_FROM,
		kind: SettingKind::Text,
		secret: false,
		default: None,
	},
	SettingSpec {
		key: SMTP_ENCRYPTION,
		kind: SettingKind::Enum(SMTP_ENCRYPTION_MODES),
		secret: false,
		default: Some(DEFAULT_SMTP_ENCRYPTION),
	},
	SettingSpec {
		key: REQUIRE_EMAIL_VERIFICATION,
		kind: SettingKind::Enum(TOGGLE_MODES),
		secret: false,
		default: Some(DEFAULT_TOGGLE),
	},
];

pub fn lookup(key: &str) -> Option<&'static SettingSpec> {
	REGISTRY.iter().find(|spec| spec.key == key)
}
