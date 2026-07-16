CREATE TABLE settings (
	key TEXT PRIMARY KEY,
	value TEXT NOT NULL,
	encrypted BIGINT NOT NULL DEFAULT 0,
	updated_at TEXT NOT NULL
);
