use super::{
	ABS_API_TOKEN, ABS_LIBRARY_ID, ABS_URL, APPRISE_URL, BASE_URL, DEFAULT_OIDC_SCOPES,
	DEFAULT_PROVIDER_TOGGLE, DEFAULT_SMTP_ENCRYPTION, DEFAULT_TOGGLE, DISCORD_WEBHOOK_URL,
	DOWNLOAD_DIR, NOTIFICATION_WEBHOOK_URL, NTFY_TOPIC_URL, OIDC_CLIENT_ID, OIDC_CLIENT_SECRET,
	OIDC_ISSUER_URL, OIDC_SCOPES, PROWLARR_API_KEY, PROWLARR_URL, QBITTORRENT_API_KEY,
	QBITTORRENT_PASSWORD, QBITTORRENT_URL, QBITTORRENT_USERNAME, REQUIRE_EMAIL_VERIFICATION,
	SABNZBD_API_KEY, SABNZBD_URL, SLACK_WEBHOOK_URL, SMTP_ENCRYPTION, SMTP_ENCRYPTION_MODES,
	SMTP_FROM, SMTP_HOST, SMTP_PASSWORD, SMTP_PORT, SMTP_USERNAME, SettingKind, SettingSpec,
	TOGGLE_MODES, TRANSMISSION_PASSWORD, TRANSMISSION_URL, TRANSMISSION_USERNAME,
};
use crate::consts::library::SETTING_LIBRARY_DIR;
use crate::consts::merge::{
	DEFAULT_FFMPEG_BINARY, DEFAULT_MERGE_BITRATE, DEFAULT_MERGE_SOURCE_ACTION,
	MERGE_SOURCE_ACTIONS, SETTING_MERGE_BACKUP_DIR, SETTING_MERGE_BITRATE, SETTING_MERGE_ENABLED,
	SETTING_MERGE_FFMPEG_PATH, SETTING_MERGE_SOURCE_ACTION,
};
use crate::consts::metadata::{
	DEFAULT_AUDNEXUS_URL, DEFAULT_METADATA_REGION, DEFAULT_PROVIDER_PRIORITY_VALUE,
	MAX_PROVIDER_PRIORITY, METADATA_AUDNEXUS_ENABLED, METADATA_AUDNEXUS_PRIORITY,
	METADATA_AUDNEXUS_URL, METADATA_GOOGLE_BOOKS_API_KEY, METADATA_GOOGLE_BOOKS_ENABLED,
	METADATA_GOOGLE_BOOKS_PRIORITY, METADATA_HARDCOVER_API_KEY, METADATA_HARDCOVER_ENABLED,
	METADATA_HARDCOVER_PRIORITY, METADATA_OPENLIBRARY_ENABLED, METADATA_OPENLIBRARY_PRIORITY,
	METADATA_REGION_SETTING, MIN_PROVIDER_PRIORITY, VALID_METADATA_REGIONS,
};

pub const REGISTRY: &[SettingSpec] = &[
	SettingSpec {
		key: METADATA_AUDNEXUS_URL,
		kind: SettingKind::Url,
		secret: false,
		default: Some(DEFAULT_AUDNEXUS_URL),
	},
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
		key: METADATA_GOOGLE_BOOKS_ENABLED,
		kind: SettingKind::Enum(TOGGLE_MODES),
		secret: false,
		default: Some(DEFAULT_PROVIDER_TOGGLE),
	},
	SettingSpec {
		key: METADATA_GOOGLE_BOOKS_PRIORITY,
		kind: SettingKind::Number {
			min: MIN_PROVIDER_PRIORITY,
			max: MAX_PROVIDER_PRIORITY,
		},
		secret: false,
		default: Some(DEFAULT_PROVIDER_PRIORITY_VALUE),
	},
	SettingSpec {
		key: METADATA_GOOGLE_BOOKS_API_KEY,
		kind: SettingKind::Text,
		secret: true,
		default: None,
	},
	SettingSpec {
		key: SETTING_MERGE_ENABLED,
		kind: SettingKind::Enum(TOGGLE_MODES),
		secret: false,
		default: Some(DEFAULT_TOGGLE),
	},
	SettingSpec {
		key: SETTING_MERGE_SOURCE_ACTION,
		kind: SettingKind::Enum(MERGE_SOURCE_ACTIONS),
		secret: false,
		default: Some(DEFAULT_MERGE_SOURCE_ACTION),
	},
	SettingSpec {
		key: SETTING_MERGE_BACKUP_DIR,
		kind: SettingKind::Path,
		secret: false,
		default: None,
	},
	SettingSpec {
		key: SETTING_MERGE_FFMPEG_PATH,
		kind: SettingKind::Text,
		secret: false,
		default: Some(DEFAULT_FFMPEG_BINARY),
	},
	SettingSpec {
		key: SETTING_MERGE_BITRATE,
		kind: SettingKind::Text,
		secret: false,
		default: Some(DEFAULT_MERGE_BITRATE),
	},
	SettingSpec {
		key: METADATA_HARDCOVER_ENABLED,
		kind: SettingKind::Enum(TOGGLE_MODES),
		secret: false,
		default: Some(DEFAULT_TOGGLE),
	},
	SettingSpec {
		key: METADATA_HARDCOVER_PRIORITY,
		kind: SettingKind::Number {
			min: MIN_PROVIDER_PRIORITY,
			max: MAX_PROVIDER_PRIORITY,
		},
		secret: false,
		default: Some(DEFAULT_PROVIDER_PRIORITY_VALUE),
	},
	SettingSpec {
		key: METADATA_HARDCOVER_API_KEY,
		kind: SettingKind::Text,
		secret: true,
		default: None,
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
		key: SABNZBD_URL,
		kind: SettingKind::Url,
		secret: false,
		default: None,
	},
	SettingSpec {
		key: SABNZBD_API_KEY,
		kind: SettingKind::Text,
		secret: true,
		default: None,
	},
	SettingSpec {
		key: OIDC_ISSUER_URL,
		kind: SettingKind::Url,
		secret: false,
		default: None,
	},
	SettingSpec {
		key: OIDC_CLIENT_ID,
		kind: SettingKind::Text,
		secret: false,
		default: None,
	},
	SettingSpec {
		key: OIDC_CLIENT_SECRET,
		kind: SettingKind::Text,
		secret: true,
		default: None,
	},
	SettingSpec {
		key: OIDC_SCOPES,
		kind: SettingKind::Text,
		secret: false,
		default: Some(DEFAULT_OIDC_SCOPES),
	},
	SettingSpec {
		key: TRANSMISSION_URL,
		kind: SettingKind::Url,
		secret: false,
		default: None,
	},
	SettingSpec {
		key: TRANSMISSION_USERNAME,
		kind: SettingKind::Text,
		secret: false,
		default: None,
	},
	SettingSpec {
		key: TRANSMISSION_PASSWORD,
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
		key: NTFY_TOPIC_URL,
		kind: SettingKind::Url,
		secret: true,
		default: None,
	},
	SettingSpec {
		key: APPRISE_URL,
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
