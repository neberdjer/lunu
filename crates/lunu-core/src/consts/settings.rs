use crate::consts::library::SETTING_LIBRARY_DIR;
use crate::consts::metadata::{
	DEFAULT_METADATA_REGION, METADATA_REGION_SETTING, VALID_METADATA_REGIONS,
};
use crate::consts::reasons;

pub const PROWLARR: &str = "prowlarr";
pub const QBITTORRENT: &str = "qbittorrent";

pub const PROWLARR_URL: &str = "prowlarr_url";
pub const PROWLARR_API_KEY: &str = "prowlarr_api_key";
pub const QBITTORRENT_URL: &str = "qbittorrent_url";
pub const QBITTORRENT_API_KEY: &str = "qbittorrent_api_key";
pub const QBITTORRENT_USERNAME: &str = "qbittorrent_username";
pub const QBITTORRENT_PASSWORD: &str = "qbittorrent_password";
pub const DOWNLOAD_DIR: &str = "download_dir";
pub const ABS_URL: &str = "abs_url";
pub const NOTIFICATION_WEBHOOK_URL: &str = "notification_webhook_url";
pub const DISCORD_WEBHOOK_URL: &str = "discord_webhook_url";
pub const SLACK_WEBHOOK_URL: &str = "slack_webhook_url";
pub const BASE_URL: &str = "base_url";
pub const SMTP_HOST: &str = "smtp_host";
pub const SMTP_PORT: &str = "smtp_port";
pub const SMTP_USERNAME: &str = "smtp_username";
pub const SMTP_PASSWORD: &str = "smtp_password";
pub const SMTP_FROM: &str = "smtp_from";
pub const SMTP_ENCRYPTION: &str = "smtp_encryption";

pub const SMTP_ENCRYPTION_MODES: &[&str] = &["starttls", "tls", "none"];
pub const DEFAULT_SMTP_ENCRYPTION: &str = "starttls";

pub enum SettingKind {
	Text,
	Url,
	Path,
	Enum(&'static [&'static str]),
}

impl SettingKind {
	pub fn as_str(&self) -> &'static str {
		match self {
			SettingKind::Text => "text",
			SettingKind::Url => "url",
			SettingKind::Path => "path",
			SettingKind::Enum(_) => "enum",
		}
	}

	pub fn choices(&self) -> &'static [&'static str] {
		match self {
			SettingKind::Enum(choices) => choices,
			_ => &[],
		}
	}
}

pub struct SettingSpec {
	pub key: &'static str,
	pub kind: SettingKind,
	pub secret: bool,
	pub default: Option<&'static str>,
}

impl SettingSpec {
	pub fn validate(&self, value: &str) -> Result<(), &'static str> {
		let trimmed = value.trim();
		if trimmed.is_empty() {
			return Err(reasons::SETTING_EMPTY);
		}
		match &self.kind {
			SettingKind::Text | SettingKind::Path => Ok(()),
			SettingKind::Url => {
				if is_http_url(trimmed) {
					Ok(())
				} else {
					Err(reasons::SETTING_INVALID_URL)
				}
			}
			SettingKind::Enum(choices) => {
				if choices.contains(&trimmed) {
					Ok(())
				} else {
					Err(reasons::SETTING_INVALID_CHOICE)
				}
			}
		}
	}
}

fn is_http_url(value: &str) -> bool {
	let rest = value
		.strip_prefix("https://")
		.or_else(|| value.strip_prefix("http://"));
	rest.is_some_and(|host| !host.is_empty() && !host.starts_with('/'))
}

pub const REGISTRY: &[SettingSpec] = &[
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
];

pub fn lookup(key: &str) -> Option<&'static SettingSpec> {
	REGISTRY.iter().find(|spec| spec.key == key)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn spec(key: &str) -> &'static SettingSpec {
		lookup(key).expect("registry has key")
	}

	#[test]
	fn url_kind_requires_http_scheme() {
		let prowlarr = spec(PROWLARR_URL);
		assert!(prowlarr.validate("http://localhost:9696").is_ok());
		assert!(prowlarr.validate("https://prowlarr.example.com").is_ok());
		assert_eq!(
			prowlarr.validate("localhost:9696"),
			Err(reasons::SETTING_INVALID_URL)
		);
		assert_eq!(
			prowlarr.validate("https://"),
			Err(reasons::SETTING_INVALID_URL)
		);
	}

	#[test]
	fn empty_value_is_rejected() {
		assert_eq!(
			spec(DOWNLOAD_DIR).validate("   "),
			Err(reasons::SETTING_EMPTY)
		);
	}

	#[test]
	fn enum_kind_checks_membership() {
		let region = spec(METADATA_REGION_SETTING);
		assert!(region.validate("us").is_ok());
		assert_eq!(region.validate("zz"), Err(reasons::SETTING_INVALID_CHOICE));
	}

	#[test]
	fn secret_keys_are_marked_secret() {
		assert!(spec(PROWLARR_API_KEY).secret);
		assert!(spec(QBITTORRENT_PASSWORD).secret);
		assert!(!spec(PROWLARR_URL).secret);
	}

	#[test]
	fn unknown_key_is_absent() {
		assert!(lookup("not_a_real_setting").is_none());
	}
}
