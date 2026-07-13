ALTER TABLE users ADD COLUMN email_verified BIGINT NOT NULL DEFAULT 0;

CREATE TABLE email_verification_tokens (
	id TEXT PRIMARY KEY,
	user_id TEXT NOT NULL UNIQUE,
	code_hash TEXT NOT NULL,
	attempts BIGINT NOT NULL DEFAULT 0,
	created_at TEXT NOT NULL,
	expires_at TEXT NOT NULL
);
