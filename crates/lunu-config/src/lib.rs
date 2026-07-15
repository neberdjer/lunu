use std::fmt;

pub const ENV_BIND: &str = "LUNU_BIND";
pub const ENV_DATABASE_URL: &str = "LUNU_DATABASE_URL";
pub const ENV_MASTER_KEY: &str = "LUNU_MASTER_KEY";
pub const ENV_WORKERS: &str = "LUNU_WORKERS";
pub const ENV_TRUSTED_PROXY_HOPS: &str = "LUNU_TRUSTED_PROXY_HOPS";
pub const ENV_TRUSTED_CLIENT_IP_HEADER: &str = "LUNU_TRUSTED_CLIENT_IP_HEADER";
pub const ENV_SECURE_COOKIES: &str = "LUNU_SECURE_COOKIES";

pub const DEFAULT_BIND: &str = "127.0.0.1:8080";
pub const DEFAULT_DATABASE_URL: &str = "sqlite://data/lunu.db?mode=rwc";
pub const MIN_MASTER_KEY_LEN: usize = 16;
pub const MAX_DEFAULT_WORKERS: usize = 8;

#[derive(Debug, Clone)]
pub struct BootstrapConfig {
	pub bind: String,
	pub database_url: String,
	pub master_key: String,
	pub workers: usize,
	pub trusted_proxy_hops: usize,
	pub trusted_client_ip_header: Option<String>,
	pub secure_cookies: bool,
}

impl BootstrapConfig {
	pub fn from_env() -> Result<Self, ConfigError> {
		let mut issues = Vec::new();

		let bind = env_or(ENV_BIND, DEFAULT_BIND);
		if !is_valid_bind(&bind) {
			issues.push(Issue {
				var: ENV_BIND,
				problem: format!("'{bind}' is not a valid host:port address"),
				hint: "expected host:port, for example 127.0.0.1:8080 or 0.0.0.0:8080",
			});
		}

		let database_url = env_or(ENV_DATABASE_URL, DEFAULT_DATABASE_URL);
		if !is_supported_database_url(&database_url) {
			issues.push(Issue {
				var: ENV_DATABASE_URL,
				problem: format!("'{database_url}' is not a supported database url"),
				hint: "expected a sqlite:// or postgres:// url, for example sqlite://data/lunu.db?mode=rwc",
			});
		}

		let workers = match std::env::var(ENV_WORKERS)
			.ok()
			.filter(|v| !v.trim().is_empty())
		{
			None => default_workers(),
			Some(value) => match value.trim().parse::<usize>() {
				Ok(count) if count > 0 => count,
				_ => {
					issues.push(Issue {
						var: ENV_WORKERS,
						problem: format!("'{value}' is not a positive integer"),
						hint: "set the number of HTTP worker threads, for example 4",
					});
					default_workers()
				}
			},
		};

		let master_key = std::env::var(ENV_MASTER_KEY).unwrap_or_default();
		if master_key.trim().is_empty() {
			issues.push(Issue {
				var: ENV_MASTER_KEY,
				problem: "is not set".to_string(),
				hint: "used to encrypt secrets at rest; set a random value of at least 16 characters. generate one with: openssl rand -base64 32",
			});
		} else if master_key.len() < MIN_MASTER_KEY_LEN {
			issues.push(Issue {
				var: ENV_MASTER_KEY,
				problem: format!(
					"must be at least {MIN_MASTER_KEY_LEN} characters (got {})",
					master_key.len()
				),
				hint: "generate a stronger value with: openssl rand -base64 32",
			});
		}

		if issues.is_empty() {
			Ok(Self {
				bind,
				database_url,
				master_key,
				workers,
				trusted_proxy_hops: env_usize(ENV_TRUSTED_PROXY_HOPS),
				trusted_client_ip_header: env_optional(ENV_TRUSTED_CLIENT_IP_HEADER),
				secure_cookies: env_flag_or(ENV_SECURE_COOKIES, true),
			})
		} else {
			Err(ConfigError { issues })
		}
	}
}

#[derive(Debug)]
pub struct ConfigError {
	issues: Vec<Issue>,
}

#[derive(Debug)]
struct Issue {
	var: &'static str,
	problem: String,
	hint: &'static str,
}

impl fmt::Display for ConfigError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		writeln!(f, "invalid configuration:")?;
		writeln!(f)?;
		for issue in &self.issues {
			writeln!(f, "  {} {}", issue.var, issue.problem)?;
			writeln!(f, "    {}", issue.hint)?;
		}
		writeln!(f)?;
		write!(
			f,
			"set these as environment variables or in a .env file (see .env.example)"
		)
	}
}

impl std::error::Error for ConfigError {}

fn env_or(key: &str, default: &str) -> String {
	std::env::var(key)
		.ok()
		.filter(|value| !value.trim().is_empty())
		.unwrap_or_else(|| default.to_string())
}

fn env_flag_or(key: &str, default: bool) -> bool {
	match std::env::var(key) {
		Ok(value) => matches!(
			value.trim().to_ascii_lowercase().as_str(),
			"1" | "true" | "yes"
		),
		Err(_) => default,
	}
}

fn env_usize(key: &str) -> usize {
	std::env::var(key)
		.ok()
		.and_then(|value| value.trim().parse::<usize>().ok())
		.unwrap_or(0)
}

fn env_optional(key: &str) -> Option<String> {
	std::env::var(key)
		.ok()
		.map(|value| value.trim().to_string())
		.filter(|value| !value.is_empty())
}

fn is_valid_bind(bind: &str) -> bool {
	match bind.rsplit_once(':') {
		Some((host, port)) => !host.is_empty() && port.parse::<u16>().is_ok(),
		None => false,
	}
}

fn is_supported_database_url(url: &str) -> bool {
	url.starts_with("sqlite:") || url.starts_with("postgres:") || url.starts_with("postgresql:")
}

fn default_workers() -> usize {
	std::thread::available_parallelism()
		.map(|count| count.get().min(MAX_DEFAULT_WORKERS))
		.unwrap_or(4)
}
