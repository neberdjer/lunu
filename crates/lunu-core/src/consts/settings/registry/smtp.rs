use super::*;

pub(super) const SMTP_SETTINGS: &[SettingSpec] = &[
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
		key: SMTP_REPLY_TO,
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
];
