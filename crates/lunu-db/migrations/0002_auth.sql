CREATE TABLE users (
	id TEXT PRIMARY KEY,
	username TEXT NOT NULL UNIQUE,
	email TEXT UNIQUE,
	password_hash TEXT,
	role TEXT NOT NULL,
	auth_source TEXT NOT NULL,
	enabled BIGINT NOT NULL DEFAULT 1,
	created_at TEXT NOT NULL,
	updated_at TEXT NOT NULL
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

CREATE TABLE settings (
	key TEXT PRIMARY KEY,
	value TEXT NOT NULL,
	encrypted BIGINT NOT NULL DEFAULT 0,
	updated_at TEXT NOT NULL
);

CREATE INDEX idx_sessions_user_id ON sessions (user_id);
CREATE INDEX idx_api_keys_user_id ON api_keys (user_id);
CREATE INDEX idx_invites_created_by ON invites (created_by);
