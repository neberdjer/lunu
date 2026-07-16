CREATE TABLE media (
	id TEXT PRIMARY KEY,
	work_id TEXT,
	format TEXT NOT NULL DEFAULT 'audiobook',
	asin TEXT UNIQUE,
	abs_item_id TEXT UNIQUE,
	title TEXT NOT NULL,
	author TEXT,
	cover_url TEXT,
	series_name TEXT,
	series_sequence TEXT,
	library_path TEXT NOT NULL,
	source TEXT NOT NULL DEFAULT 'request',
	overridden BIGINT NOT NULL DEFAULT 0,
	request_id TEXT,
	created_at TEXT NOT NULL
);
