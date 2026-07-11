CREATE TABLE jobs (
	id TEXT PRIMARY KEY,
	job_type TEXT NOT NULL,
	payload TEXT NOT NULL,
	status TEXT NOT NULL,
	attempts BIGINT NOT NULL,
	max_attempts BIGINT NOT NULL,
	run_after TEXT NOT NULL,
	locked_by TEXT,
	locked_at TEXT,
	last_error TEXT,
	created_at TEXT NOT NULL,
	updated_at TEXT NOT NULL
);

CREATE INDEX idx_jobs_claim ON jobs (status, run_after);
