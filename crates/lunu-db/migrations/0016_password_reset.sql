CREATE TABLE password_reset_tokens (
	id TEXT PRIMARY KEY,
	user_id TEXT NOT NULL UNIQUE,
	code_hash TEXT NOT NULL,
	attempts BIGINT NOT NULL DEFAULT 0,
	created_at TEXT NOT NULL,
	expires_at TEXT NOT NULL
);
