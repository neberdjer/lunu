use super::*;

pub(super) const METADATA_SETTINGS: &[SettingSpec] = &[
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
		key: METADATA_REGION_SETTING,
		kind: SettingKind::Enum(VALID_METADATA_REGIONS),
		secret: false,
		default: Some(DEFAULT_METADATA_REGION),
	},
];
