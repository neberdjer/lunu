CREATE TABLE mfa_recovery_codes (
	id TEXT PRIMARY KEY,
	user_id TEXT NOT NULL,
	code_hash TEXT NOT NULL,
	used_at TEXT,
	created_at TEXT NOT NULL
);
CREATE INDEX idx_mfa_recovery_codes_user_id ON mfa_recovery_codes (user_id);
