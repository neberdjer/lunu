use crate::consts::reasons;

pub const FFMPEG: &str = "ffmpeg";
pub const PROWLARR: &str = "prowlarr";
pub const QBITTORRENT: &str = "qbittorrent";
pub const SABNZBD: &str = "sabnzbd";
pub const TRANSMISSION: &str = "transmission";

pub const PROWLARR_URL: &str = "prowlarr_url";
pub const PROWLARR_API_KEY: &str = "prowlarr_api_key";
pub const QBITTORRENT_URL: &str = "qbittorrent_url";
pub const QBITTORRENT_API_KEY: &str = "qbittorrent_api_key";
pub const SABNZBD_URL: &str = "sabnzbd_url";
pub const SABNZBD_API_KEY: &str = "sabnzbd_api_key";
pub const OIDC_ISSUER_URL: &str = "oidc_issuer_url";
pub const OIDC_CLIENT_ID: &str = "oidc_client_id";
pub const OIDC_CLIENT_SECRET: &str = "oidc_client_secret";
pub const OIDC_SCOPES: &str = "oidc_scopes";
pub const DEFAULT_OIDC_SCOPES: &str = "openid profile email";
pub const TRANSMISSION_URL: &str = "transmission_url";
pub const TRANSMISSION_USERNAME: &str = "transmission_username";
pub const TRANSMISSION_PASSWORD: &str = "transmission_password";
pub const QBITTORRENT_USERNAME: &str = "qbittorrent_username";
pub const QBITTORRENT_PASSWORD: &str = "qbittorrent_password";
pub const DOWNLOAD_DIR: &str = "download_dir";
pub const ABS_URL: &str = "abs_url";
pub const ABS_API_TOKEN: &str = "abs_api_token";
pub const ABS_LIBRARY_ID: &str = "abs_library_id";
pub const NOTIFICATION_WEBHOOK_URL: &str = "notification_webhook_url";
pub const DISCORD_WEBHOOK_URL: &str = "discord_webhook_url";
pub const SLACK_WEBHOOK_URL: &str = "slack_webhook_url";
pub const NTFY_TOPIC_URL: &str = "ntfy_topic_url";
pub const APPRISE_URL: &str = "apprise_url";
pub const BASE_URL: &str = "base_url";
pub const SMTP_HOST: &str = "smtp_host";
pub const SMTP_PORT: &str = "smtp_port";
pub const SMTP_USERNAME: &str = "smtp_username";
pub const SMTP_PASSWORD: &str = "smtp_password";
pub const SMTP_FROM: &str = "smtp_from";
pub const SMTP_ENCRYPTION: &str = "smtp_encryption";

pub const SMTP_ENCRYPTION_STARTTLS: &str = "starttls";
pub const SMTP_ENCRYPTION_TLS: &str = "tls";
pub const SMTP_ENCRYPTION_NONE: &str = "none";
pub const SMTP_ENCRYPTION_MODES: &[&str] = &[
	SMTP_ENCRYPTION_STARTTLS,
	SMTP_ENCRYPTION_TLS,
	SMTP_ENCRYPTION_NONE,
];
pub const DEFAULT_SMTP_ENCRYPTION: &str = SMTP_ENCRYPTION_STARTTLS;

pub const REQUIRE_EMAIL_VERIFICATION: &str = "require_email_verification";
pub const TOGGLE_ON: &str = "on";
pub const TOGGLE_OFF: &str = "off";
pub const TOGGLE_MODES: &[&str] = &[TOGGLE_OFF, TOGGLE_ON];
pub const DEFAULT_TOGGLE: &str = TOGGLE_OFF;
pub const DEFAULT_PROVIDER_TOGGLE: &str = TOGGLE_ON;

mod registry;

pub use registry::{REGISTRY, lookup};

pub enum SettingKind {
	Text,
	Url,
	Path,
	Enum(&'static [&'static str]),
	Number { min: i64, max: i64 },
}

impl SettingKind {
	pub fn as_str(&self) -> &'static str {
		match self {
			SettingKind::Text => "text",
			SettingKind::Url => "url",
			SettingKind::Path => "path",
			SettingKind::Enum(_) => "enum",
			SettingKind::Number { .. } => "number",
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
			SettingKind::Number { min, max } => match trimmed.parse::<i64>() {
				Ok(number) if (*min..=*max).contains(&number) => Ok(()),
				_ => Err(reasons::SETTING_OUT_OF_RANGE),
			},
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::consts::metadata::METADATA_REGION_SETTING;

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

	#[test]
	fn every_provider_setting_is_registered() {
		for settings in crate::consts::metadata::METADATA_PROVIDER_SETTINGS {
			let provider = settings.provider;
			for key in [settings.enabled, settings.priority] {
				let spec = lookup(key).unwrap_or_else(|| {
					panic!(
						"provider {provider} maps to setting {key}, which is not in the registry: it would be unsettable through the api and invisible in the catalog"
					)
				});
				assert!(
					spec.default.is_some(),
					"provider setting {key} needs a default, or a fresh install has no opinion on it"
				);
			}

			let enabled = lookup(settings.enabled).expect("registered above");
			assert!(
				matches!(enabled.kind, SettingKind::Enum(TOGGLE_MODES)),
				"provider toggle {} must be an on/off setting",
				settings.enabled
			);

			let priority = lookup(settings.priority).expect("registered above");
			assert!(
				matches!(priority.kind, SettingKind::Number { .. }),
				"provider priority {} must be a number",
				settings.priority
			);
			assert!(
				priority
					.default
					.is_some_and(|value| priority.validate(value).is_ok()),
				"provider priority {} declares a default outside its own range",
				settings.priority
			);
		}
	}

	#[test]
	fn the_provider_priority_default_agrees_with_itself() {
		use crate::consts::metadata::{DEFAULT_PROVIDER_PRIORITY, DEFAULT_PROVIDER_PRIORITY_VALUE};
		assert_eq!(
			DEFAULT_PROVIDER_PRIORITY_VALUE.parse::<i64>(),
			Ok(DEFAULT_PROVIDER_PRIORITY),
			"the registry default and the priority the service falls back to are the same number written twice"
		);
	}

	#[test]
	fn number_kind_enforces_its_range() {
		let priority = spec(crate::consts::metadata::METADATA_AUDNEXUS_PRIORITY);
		assert!(priority.validate("1").is_ok());
		assert!(priority.validate("100").is_ok());
		assert_eq!(priority.validate("0"), Err(reasons::SETTING_OUT_OF_RANGE));
		assert_eq!(priority.validate("101"), Err(reasons::SETTING_OUT_OF_RANGE));
		assert_eq!(priority.validate("abc"), Err(reasons::SETTING_OUT_OF_RANGE));
		assert_eq!(priority.validate("1.5"), Err(reasons::SETTING_OUT_OF_RANGE));
	}
}
