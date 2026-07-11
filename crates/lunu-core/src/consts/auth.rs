pub const SESSION_TTL_DAYS: i64 = 30;
pub const SESSION_COOKIE: &str = "lunu_session";

pub const PASSWORD_MIN_LEN: usize = 8;

pub const SCOPE_ADMIN: &str = "admin";
pub const KNOWN_API_KEY_SCOPES: &[&str] = &[SCOPE_ADMIN];

pub const API_KEY_PREFIX: &str = "lunu";
pub const API_KEY_DISPLAY_LEN: usize = 12;
pub const API_KEY_HEADER: &str = "x-api-key";
pub const BEARER_PREFIX: &str = "Bearer ";

pub const DEFAULT_INVITE_MAX_USES: i64 = 1;

pub const AUTH_RATE_LIMIT_ATTEMPTS: u32 = 10;
pub const AUTH_RATE_LIMIT_WINDOW_SECS: u64 = 60;
