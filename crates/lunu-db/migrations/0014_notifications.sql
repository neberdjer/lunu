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
