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
	updated_at TEXT NOT NULL,
	request_id TEXT
);
CREATE UNIQUE INDEX idx_jobs_active_recurring ON jobs (job_type)
	WHERE request_id IS NULL
	AND status IN ('pending', 'running')
	AND job_type IN ('library-sync', 'session-cleanup', 'job-cleanup');
CREATE INDEX idx_jobs_claim ON jobs (status, run_after);
CREATE INDEX idx_jobs_request_id ON jobs (request_id);

CREATE TABLE schedules (
	kind TEXT PRIMARY KEY,
	interval_secs BIGINT NOT NULL,
	enabled BIGINT NOT NULL DEFAULT 0,
	next_run_at TEXT NOT NULL,
	last_run_at TEXT,
	updated_at TEXT NOT NULL
);
