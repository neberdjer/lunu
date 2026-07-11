CREATE TABLE downloads (
	id TEXT PRIMARY KEY,
	request_id TEXT NOT NULL,
	client TEXT NOT NULL,
	category TEXT NOT NULL,
	release_title TEXT NOT NULL,
	indexer TEXT NOT NULL,
	download_url TEXT NOT NULL,
	state TEXT NOT NULL,
	created_at TEXT NOT NULL,
	updated_at TEXT NOT NULL
);

CREATE INDEX idx_downloads_request_id ON downloads (request_id);
