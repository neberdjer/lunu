CREATE TABLE users (
	id TEXT PRIMARY KEY,
	username TEXT NOT NULL UNIQUE,
	email TEXT UNIQUE,
	password_hash TEXT,
	role TEXT NOT NULL,
	auth_source TEXT NOT NULL,
	oidc_subject TEXT UNIQUE,
	enabled BIGINT NOT NULL DEFAULT 1,
	created_at TEXT NOT NULL,
	updated_at TEXT NOT NULL,
	display_name TEXT,
	locale TEXT,
	email_verified BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE sessions (
	id TEXT PRIMARY KEY,
	user_id TEXT NOT NULL,
	token_hash TEXT NOT NULL UNIQUE,
	created_at TEXT NOT NULL,
	expires_at TEXT NOT NULL,
	last_seen_at TEXT,
	user_agent TEXT
);
CREATE INDEX idx_sessions_user_id ON sessions (user_id);

CREATE TABLE api_keys (
	id TEXT PRIMARY KEY,
	user_id TEXT NOT NULL,
	name TEXT NOT NULL,
	prefix TEXT NOT NULL,
	key_hash TEXT NOT NULL UNIQUE,
	scopes TEXT NOT NULL,
	created_at TEXT NOT NULL,
	last_used_at TEXT,
	expires_at TEXT,
	revoked BIGINT NOT NULL DEFAULT 0
);
CREATE INDEX idx_api_keys_user_id ON api_keys (user_id);

CREATE TABLE invites (
	id TEXT PRIMARY KEY,
	code_hash TEXT NOT NULL UNIQUE,
	role TEXT NOT NULL,
	email TEXT,
	created_by TEXT NOT NULL,
	max_uses BIGINT NOT NULL DEFAULT 1,
	used_count BIGINT NOT NULL DEFAULT 0,
	created_at TEXT NOT NULL,
	expires_at TEXT
);

CREATE TABLE password_reset_tokens (
	id TEXT PRIMARY KEY,
	user_id TEXT NOT NULL UNIQUE,
	code_hash TEXT NOT NULL,
	attempts BIGINT NOT NULL DEFAULT 0,
	created_at TEXT NOT NULL,
	expires_at TEXT NOT NULL
);

CREATE TABLE email_verification_tokens (
	id TEXT PRIMARY KEY,
	user_id TEXT NOT NULL UNIQUE,
	code_hash TEXT NOT NULL,
	attempts BIGINT NOT NULL DEFAULT 0,
	created_at TEXT NOT NULL,
	expires_at TEXT NOT NULL
);

CREATE TABLE user_mfa (
	user_id TEXT PRIMARY KEY,
	method TEXT NOT NULL,
	secret TEXT,
	confirmed BIGINT NOT NULL DEFAULT 0,
	created_at TEXT NOT NULL,
	updated_at TEXT NOT NULL
);

CREATE TABLE user_settings (
	user_id TEXT PRIMARY KEY,
	auto_approve BIGINT NOT NULL DEFAULT 0,
	request_quota BIGINT,
	quota_days BIGINT,
	updated_at TEXT NOT NULL
);

CREATE TABLE mfa_recovery_codes (
	id TEXT PRIMARY KEY,
	user_id TEXT NOT NULL,
	code_hash TEXT NOT NULL,
	used_at TEXT,
	created_at TEXT NOT NULL
);
CREATE INDEX idx_mfa_recovery_codes_user_id ON mfa_recovery_codes (user_id);
CREATE INDEX idx_sessions_expires_at ON sessions (expires_at);
