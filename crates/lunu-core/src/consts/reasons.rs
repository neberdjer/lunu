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
pub const SABNZBD_NOT_CONFIGURED: &str = "sabnzbd-not-configured";
pub const SMTP_NOT_CONFIGURED: &str = "smtp-not-configured";
pub const TRANSMISSION_NOT_CONFIGURED: &str = "transmission-not-configured";
pub const NO_CLIENT_FOR_PROTOCOL: &str = "no-client-for-protocol";
pub const INVALID_REGION: &str = "invalid-region";
pub const ALREADY_EXISTS: &str = "already-exists";
pub const NO_RELEASES: &str = "no-releases";
pub const DOWNLOAD_STATE_UNKNOWN: &str = "download-state-unknown";
pub const PROTOCOL_UNKNOWN: &str = "protocol-unknown";
pub const PROTOCOL_REQUIRED: &str = "protocol-required";
pub const INVALID_LOG_LEVEL: &str = "invalid-log-level";
pub const OIDC_NOT_CONFIGURED: &str = "oidc-not-configured";
pub const OIDC_STATE_INVALID: &str = "oidc-state-invalid";
pub const OIDC_ACCOUNT_CONFLICT: &str = "oidc-account-conflict";
pub const MERGE_UNAVAILABLE: &str = "merge-unavailable";
pub const MERGE_SOURCE_ACTION_UNKNOWN: &str = "merge-source-action-unknown";
pub const MERGE_BACKUP_NOT_CONFIGURED: &str = "merge-backup-not-configured";
pub const MFA_METHOD_UNKNOWN: &str = "mfa-method-unknown";
pub const MFA_REQUIRED: &str = "mfa-required";
pub const MFA_TICKET_INVALID: &str = "mfa-ticket-invalid";
pub const MFA_CODE_INVALID: &str = "mfa-code-invalid";
pub const MFA_ALREADY_ENABLED: &str = "mfa-already-enabled";
pub const MFA_NOT_ENROLLED: &str = "mfa-not-enrolled";
pub const MFA_EMAIL_REQUIRED: &str = "mfa-email-required";
pub const JOB_TYPE_UNKNOWN: &str = "job-type-unknown";
pub const ID_SCHEME_UNKNOWN: &str = "id-scheme-unknown";
pub const FORMAT_UNKNOWN: &str = "format-unknown";
pub const MATCH_KIND_UNKNOWN: &str = "match-kind-unknown";
pub const NO_PROVIDER_FOR_ID: &str = "no-provider-for-id";
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
pub const SETTING_OUT_OF_RANGE: &str = "setting-out-of-range";
pub const NOTIFICATION_KIND_UNKNOWN: &str = "notification-kind-unknown";
pub const ISSUE_STATUS_UNKNOWN: &str = "issue-status-unknown";
pub const ISSUE_TYPE_UNKNOWN: &str = "issue-type-unknown";
pub const ISSUE_NOT_OPEN: &str = "issue-not-open";
pub const REQUEST_NOT_AVAILABLE: &str = "request-not-available";
pub const UNKNOWN_PROFILE: &str = "unknown-profile";
pub const JOB_NOT_RETRYABLE: &str = "job-not-retryable";
pub const EMPTY_SELECTION: &str = "empty-selection";
pub const TOO_MANY_ITEMS: &str = "too-many-items";
pub const USERNAME_INVALID: &str = "username-invalid";
pub const DOWNLOAD_IN_PROGRESS: &str = "download-in-progress";
pub const PROFILE_NAME_REQUIRED: &str = "profile-name-required";
pub const RESET_TOKEN_INVALID: &str = "reset-token-invalid";
pub const UNSUBSCRIBE_TOKEN_INVALID: &str = "unsubscribe-token-invalid";
pub const EMAIL_NOT_VERIFIED: &str = "email-not-verified";
pub const VERIFICATION_INVALID: &str = "verification-invalid";
pub const MEDIA_FILTER_UNKNOWN: &str = "media-filter-unknown";
pub const MERGE_STATE_UNKNOWN: &str = "merge-state-unknown";
pub const IMPORT_UNLISTED_ACTION_UNKNOWN: &str = "import-unlisted-action-unknown";
pub const MERGE_NOTHING_TO_REVERT: &str = "merge-nothing-to-revert";
pub const INVALID_EXPIRY: &str = "invalid-expiry";
pub const MEDIA_SOURCE_UNKNOWN: &str = "media-source-unknown";
pub const NO_METADATA_PROVIDER: &str = "no-metadata-provider";
pub const ABS_NOT_CONFIGURED: &str = "abs-not-configured";
pub const ABS_UNAUTHORIZED: &str = "abs-unauthorized";
pub const REQUEST_TITLE_REQUIRED: &str = "request-title-required";
pub const QUOTA_INVALID: &str = "quota-invalid";
pub const SCHEDULE_INTERVAL_INVALID: &str = "schedule-interval-invalid";
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
	SABNZBD_NOT_CONFIGURED,
	SMTP_NOT_CONFIGURED,
	TRANSMISSION_NOT_CONFIGURED,
	NO_CLIENT_FOR_PROTOCOL,
	INVALID_REGION,
	ALREADY_EXISTS,
	NO_RELEASES,
	DOWNLOAD_STATE_UNKNOWN,
	PROTOCOL_UNKNOWN,
	PROTOCOL_REQUIRED,
	INVALID_LOG_LEVEL,
	OIDC_NOT_CONFIGURED,
	OIDC_STATE_INVALID,
	OIDC_ACCOUNT_CONFLICT,
	MERGE_UNAVAILABLE,
	MERGE_SOURCE_ACTION_UNKNOWN,
	MERGE_BACKUP_NOT_CONFIGURED,
	MFA_METHOD_UNKNOWN,
	MFA_REQUIRED,
	MFA_TICKET_INVALID,
	MFA_CODE_INVALID,
	MFA_ALREADY_ENABLED,
	MFA_NOT_ENROLLED,
	MFA_EMAIL_REQUIRED,
	JOB_TYPE_UNKNOWN,
	ID_SCHEME_UNKNOWN,
	FORMAT_UNKNOWN,
	MATCH_KIND_UNKNOWN,
	NO_PROVIDER_FOR_ID,
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
	SETTING_OUT_OF_RANGE,
	NOTIFICATION_KIND_UNKNOWN,
	ISSUE_STATUS_UNKNOWN,
	ISSUE_TYPE_UNKNOWN,
	ISSUE_NOT_OPEN,
	REQUEST_NOT_AVAILABLE,
	UNKNOWN_PROFILE,
	JOB_NOT_RETRYABLE,
	EMPTY_SELECTION,
	TOO_MANY_ITEMS,
	USERNAME_INVALID,
	DOWNLOAD_IN_PROGRESS,
	PROFILE_NAME_REQUIRED,
	RESET_TOKEN_INVALID,
	UNSUBSCRIBE_TOKEN_INVALID,
	EMAIL_NOT_VERIFIED,
	VERIFICATION_INVALID,
	MEDIA_FILTER_UNKNOWN,
	MERGE_STATE_UNKNOWN,
	IMPORT_UNLISTED_ACTION_UNKNOWN,
	MERGE_NOTHING_TO_REVERT,
	INVALID_EXPIRY,
	MEDIA_SOURCE_UNKNOWN,
	NO_METADATA_PROVIDER,
	ABS_NOT_CONFIGURED,
	ABS_UNAUTHORIZED,
	REQUEST_TITLE_REQUIRED,
	QUOTA_INVALID,
	SCHEDULE_INTERVAL_INVALID,
	UNKNOWN_LOCALE,
];
