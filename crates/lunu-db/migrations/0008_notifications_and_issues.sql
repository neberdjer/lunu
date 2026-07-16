CREATE TABLE user_notifications (
	id TEXT PRIMARY KEY,
	user_id TEXT NOT NULL,
	kind TEXT NOT NULL,
	request_id TEXT,
	title TEXT NOT NULL,
	created_at TEXT NOT NULL,
	read_at TEXT
);
CREATE INDEX idx_user_notifications_user ON user_notifications (user_id, created_at);

CREATE TABLE issues (
	id TEXT PRIMARY KEY,
	request_id TEXT NOT NULL,
	reporter_id TEXT NOT NULL,
	issue_type TEXT NOT NULL,
	detail TEXT,
	status TEXT NOT NULL,
	resolved_by TEXT,
	created_at TEXT NOT NULL,
	updated_at TEXT NOT NULL
);
CREATE INDEX idx_issues_request ON issues (request_id);
CREATE INDEX idx_issues_status ON issues (status);
