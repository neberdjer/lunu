CREATE TABLE media (
	asin TEXT PRIMARY KEY,
	title TEXT NOT NULL,
	author TEXT,
	cover_url TEXT,
	library_path TEXT NOT NULL,
	request_id TEXT,
	created_at TEXT NOT NULL
);
