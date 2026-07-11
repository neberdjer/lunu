CREATE TABLE requests (
	id TEXT PRIMARY KEY,
	user_id TEXT NOT NULL,
	asin TEXT NOT NULL,
	title TEXT NOT NULL,
	author TEXT,
	cover_url TEXT,
	status TEXT NOT NULL,
	approved_by TEXT,
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

CREATE INDEX idx_requests_user_id ON requests (user_id);
CREATE INDEX idx_requests_asin ON requests (asin);
