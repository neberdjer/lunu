CREATE TABLE schedules (
	kind TEXT PRIMARY KEY,
	interval_secs BIGINT NOT NULL,
	enabled BIGINT NOT NULL DEFAULT 0,
	next_run_at TEXT NOT NULL,
	last_run_at TEXT,
	updated_at TEXT NOT NULL
);
