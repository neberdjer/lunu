pub const USERNAME_TAKEN: &str = "username-taken";
pub const SETUP_COMPLETED: &str = "setup-completed";
pub const INVITE_INVALID: &str = "invite-invalid";
pub const INVITE_UNUSABLE: &str = "invite-unusable";
pub const INVITE_MAX_USES: &str = "invite-max-uses";
pub const ROLE_UNKNOWN: &str = "role-unknown";
pub const AUTH_SOURCE_UNKNOWN: &str = "auth-source-unknown";
pub const REQUEST_STATUS_UNKNOWN: &str = "request-status-unknown";
pub const INVALID_ASIN: &str = "invalid-asin";
pub const ALREADY_REQUESTED: &str = "already-requested";
pub const QUOTA_EXCEEDED: &str = "quota-exceeded";
pub const REQUEST_NOT_PENDING: &str = "request-not-pending";
pub const REQUEST_NOT_RETRYABLE: &str = "request-not-retryable";
pub const PROWLARR_NOT_CONFIGURED: &str = "prowlarr-not-configured";
pub const PROWLARR_UNAUTHORIZED: &str = "prowlarr-unauthorized";
pub const QBITTORRENT_NOT_CONFIGURED: &str = "qbittorrent-not-configured";
pub const QBITTORRENT_AUTH_FAILED: &str = "qbittorrent-auth-failed";
pub const INVALID_REGION: &str = "invalid-region";
pub const ALREADY_EXISTS: &str = "already-exists";
pub const NO_RELEASES: &str = "no-releases";
pub const DOWNLOAD_STATE_UNKNOWN: &str = "download-state-unknown";
pub const JOB_TYPE_UNKNOWN: &str = "job-type-unknown";
pub const JOB_STATUS_UNKNOWN: &str = "job-status-unknown";
pub const LIBRARY_NOT_CONFIGURED: &str = "library-not-configured";
pub const LAST_ADMIN: &str = "last-admin";
pub const UNKNOWN_SCOPE: &str = "unknown-scope";
pub const PASSWORD_TOO_SHORT: &str = "password-too-short";
pub const PASSWORD_NOT_LOCAL: &str = "password-not-local";
pub const EMAIL_INVALID: &str = "email-invalid";
pub const UNKNOWN_SETTING: &str = "unknown-setting";
pub const SETTING_EMPTY: &str = "setting-empty";
pub const SETTING_INVALID_URL: &str = "setting-invalid-url";
pub const SETTING_INVALID_CHOICE: &str = "setting-invalid-choice";
pub const NOTIFICATION_KIND_UNKNOWN: &str = "notification-kind-unknown";
pub const ISSUE_STATUS_UNKNOWN: &str = "issue-status-unknown";
pub const ISSUE_TYPE_UNKNOWN: &str = "issue-type-unknown";
pub const ISSUE_NOT_OPEN: &str = "issue-not-open";
pub const REQUEST_NOT_AVAILABLE: &str = "request-not-available";
pub const UNKNOWN_PROFILE: &str = "unknown-profile";
pub const JOB_NOT_RETRYABLE: &str = "job-not-retryable";
pub const EMPTY_SELECTION: &str = "empty-selection";
pub const RESET_TOKEN_INVALID: &str = "reset-token-invalid";
pub const UNKNOWN_LOCALE: &str = "unknown-locale";

pub const ALL: &[&str] = &[
	USERNAME_TAKEN,
	SETUP_COMPLETED,
	INVITE_INVALID,
	INVITE_UNUSABLE,
	INVITE_MAX_USES,
	ROLE_UNKNOWN,
	AUTH_SOURCE_UNKNOWN,
	REQUEST_STATUS_UNKNOWN,
	INVALID_ASIN,
	ALREADY_REQUESTED,
	QUOTA_EXCEEDED,
	REQUEST_NOT_PENDING,
	REQUEST_NOT_RETRYABLE,
	PROWLARR_NOT_CONFIGURED,
	PROWLARR_UNAUTHORIZED,
	QBITTORRENT_NOT_CONFIGURED,
	QBITTORRENT_AUTH_FAILED,
	INVALID_REGION,
	ALREADY_EXISTS,
	NO_RELEASES,
	DOWNLOAD_STATE_UNKNOWN,
	JOB_TYPE_UNKNOWN,
	JOB_STATUS_UNKNOWN,
	LIBRARY_NOT_CONFIGURED,
	LAST_ADMIN,
	UNKNOWN_SCOPE,
	PASSWORD_TOO_SHORT,
	PASSWORD_NOT_LOCAL,
	EMAIL_INVALID,
	UNKNOWN_SETTING,
	SETTING_EMPTY,
	SETTING_INVALID_URL,
	SETTING_INVALID_CHOICE,
	NOTIFICATION_KIND_UNKNOWN,
	ISSUE_STATUS_UNKNOWN,
	ISSUE_TYPE_UNKNOWN,
	ISSUE_NOT_OPEN,
	REQUEST_NOT_AVAILABLE,
	UNKNOWN_PROFILE,
	JOB_NOT_RETRYABLE,
	EMPTY_SELECTION,
	RESET_TOKEN_INVALID,
	UNKNOWN_LOCALE,
];
