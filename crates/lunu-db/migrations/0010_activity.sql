CREATE TABLE activity (
	id TEXT PRIMARY KEY,
	request_id TEXT NOT NULL,
	event TEXT NOT NULL,
	detail TEXT,
	at TEXT NOT NULL
);

CREATE INDEX idx_activity_at ON activity (at);
CREATE INDEX idx_activity_request_id ON activity (request_id);
