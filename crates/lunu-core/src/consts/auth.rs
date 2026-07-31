pub const SESSION_TTL_DAYS: i64 = 30;
pub const SESSION_COOKIE: &str = "lunu_session";

pub const PASSWORD_MIN_LEN: usize = 8;
pub const PASSWORD_MAX_LEN: usize = 1024;
pub const USERNAME_MAX_LEN: usize = 64;
pub const PASSWORD_RESET_TTL_MINUTES: i64 = 15;
pub const PASSWORD_RESET_CODE_DIGITS: u32 = 6;
pub const PASSWORD_RESET_MAX_ATTEMPTS: i64 = 5;
pub const PASSWORD_RESET_COOLDOWN_SECONDS: i64 = 120;

pub const EMAIL_VERIFICATION_TTL_MINUTES: i64 = 60;
pub const EMAIL_VERIFICATION_CODE_DIGITS: u32 = 6;
pub const EMAIL_VERIFICATION_MAX_ATTEMPTS: i64 = 5;
pub const EMAIL_VERIFICATION_COOLDOWN_SECONDS: i64 = 120;

pub const SCOPE_READ: &str = "read";
pub const SCOPE_WRITE: &str = "write";
pub const SCOPE_ADMIN: &str = "admin";
pub const KNOWN_API_KEY_SCOPES: &[&str] = &[SCOPE_READ, SCOPE_WRITE, SCOPE_ADMIN];

pub const API_KEY_PREFIX: &str = "lunu";
pub const API_KEY_DISPLAY_LEN: usize = 12;
pub const API_KEY_HEADER: &str = "x-api-key";
pub const BEARER_PREFIX: &str = "Bearer ";

pub const DEFAULT_INVITE_MAX_USES: i64 = 1;

pub const AUTH_RATE_LIMIT_ATTEMPTS: u32 = 10;
pub const AUTH_RATE_LIMIT_WINDOW_SECS: u64 = 60;

pub const METADATA_RATE_LIMIT_ATTEMPTS: u32 = 60;
pub const METADATA_RATE_LIMIT_WINDOW_SECS: u64 = 60;

pub const OIDC_STATE_TTL_MINS: i64 = 10;

pub const TOTP_SECRET_BYTES: usize = 20;
pub const TOTP_DIGITS: u32 = 6;
pub const TOTP_STEP_SECONDS: u64 = 30;
pub const TOTP_ISSUER: &str = "Lunu";

pub const MFA_CODE_DIGITS: u32 = 6;
pub const MFA_TICKET_TTL_MINUTES: i64 = 5;
pub const MFA_MAX_ATTEMPTS: i64 = 5;
pub const MFA_RECOVERY_CODE_COUNT: usize = 10;

pub const SESSION_TOUCH_INTERVAL_SECS: i64 = 300;
pub const API_KEY_TOUCH_INTERVAL_SECS: i64 = 300;
