use super::{
	ABS_API_TOKEN, ABS_LIBRARY_ID, ABS_URL, APPRISE_URL, BASE_URL, DEFAULT_OIDC_SCOPES,
	DEFAULT_PROVIDER_TOGGLE, DEFAULT_SMTP_ENCRYPTION, DEFAULT_TOGGLE, DISCORD_WEBHOOK_URL,
	DOWNLOAD_DIR, NOTIFICATION_WEBHOOK_URL, NTFY_TOPIC_URL, OIDC_CLIENT_ID, OIDC_CLIENT_SECRET,
	OIDC_ISSUER_URL, OIDC_SCOPES, PROWLARR_API_KEY, PROWLARR_URL, QBITTORRENT_API_KEY,
	QBITTORRENT_PASSWORD, QBITTORRENT_URL, QBITTORRENT_USERNAME, REQUIRE_EMAIL_VERIFICATION,
	SABNZBD_API_KEY, SABNZBD_URL, SLACK_WEBHOOK_URL, SMTP_ENCRYPTION, SMTP_ENCRYPTION_MODES,
	SMTP_FROM, SMTP_HOST, SMTP_PASSWORD, SMTP_PORT, SMTP_USERNAME, SettingKind, SettingSpec,
	TOGGLE_MODES, TOGGLE_ON, TRANSMISSION_PASSWORD, TRANSMISSION_URL, TRANSMISSION_USERNAME,
};
use crate::consts::download::SETTING_REMOVE_FAILED_DOWNLOADS;
use crate::consts::library::{
	DEFAULT_IMPORT_KEEP_EXTENSIONS, DEFAULT_IMPORT_UNLISTED, IMPORT_UNLISTED_ACTIONS,
	SETTING_IMPORT_KEEP_EXTENSIONS, SETTING_IMPORT_UNLISTED, SETTING_LIBRARY_DIR,
	SETTING_WRITE_SIDECAR,
};
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

mod metadata;

use metadata::METADATA_SETTINGS;

const CORE_SETTINGS: &[SettingSpec] = &[
	SettingSpec {
		key: SETTING_REMOVE_FAILED_DOWNLOADS,
		kind: SettingKind::Enum(TOGGLE_MODES),
		secret: false,
		default: Some(TOGGLE_ON),
	},
	SettingSpec {
		key: SETTING_IMPORT_KEEP_EXTENSIONS,
		kind: SettingKind::Text,
		secret: false,
		default: Some(DEFAULT_IMPORT_KEEP_EXTENSIONS),
	},
	SettingSpec {
		key: SETTING_IMPORT_UNLISTED,
		kind: SettingKind::Enum(IMPORT_UNLISTED_ACTIONS),
		secret: false,
		default: Some(DEFAULT_IMPORT_UNLISTED),
	},
	SettingSpec {
		key: SETTING_WRITE_SIDECAR,
		kind: SettingKind::Enum(TOGGLE_MODES),
		secret: false,
		default: Some(TOGGLE_ON),
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

pub fn registry() -> impl Iterator<Item = &'static SettingSpec> {
	CORE_SETTINGS.iter().chain(METADATA_SETTINGS.iter())
}

pub fn lookup(key: &str) -> Option<&'static SettingSpec> {
	registry().find(|spec| spec.key == key)
}
