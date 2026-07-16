CREATE TABLE requests (
	id TEXT PRIMARY KEY,
	user_id TEXT NOT NULL,
	work_id TEXT NOT NULL,
	format TEXT NOT NULL DEFAULT 'audiobook',
	asin TEXT,
	title TEXT NOT NULL,
	author TEXT,
	cover_url TEXT,
	status TEXT NOT NULL,
	approved_by TEXT,
	created_at TEXT NOT NULL,
	updated_at TEXT NOT NULL,
	notes TEXT,
	quality_profile_id TEXT
);
CREATE UNIQUE INDEX idx_requests_active ON requests (user_id, work_id, format)
	WHERE status NOT IN ('declined', 'failed');
CREATE INDEX idx_requests_user_id ON requests (user_id);
CREATE INDEX idx_requests_user_work ON requests (user_id, work_id);

CREATE TABLE activity (
	id TEXT PRIMARY KEY,
	request_id TEXT NOT NULL,
	event TEXT NOT NULL,
	detail TEXT,
	at TEXT NOT NULL,
	actor TEXT
);
CREATE INDEX idx_activity_at ON activity (at);
CREATE INDEX idx_activity_request_id ON activity (request_id);
