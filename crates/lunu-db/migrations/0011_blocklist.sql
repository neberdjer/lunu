CREATE TABLE blocklist (
	id TEXT PRIMARY KEY,
	request_id TEXT NOT NULL,
	download_url TEXT NOT NULL,
	created_at TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_blocklist_request_url ON blocklist (request_id, download_url);
