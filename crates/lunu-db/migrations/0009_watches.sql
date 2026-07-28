CREATE TABLE watches (
	id TEXT PRIMARY KEY,
	user_id TEXT NOT NULL,
	work_id TEXT NOT NULL,
	format TEXT NOT NULL DEFAULT 'audiobook',
	asin TEXT,
	title TEXT NOT NULL,
	author TEXT,
	cover_url TEXT,
	series_name TEXT,
	series_sequence TEXT,
	created_at TEXT NOT NULL
);
CREATE UNIQUE INDEX idx_watches_active ON watches (user_id, work_id, format);
CREATE INDEX idx_watches_user_created ON watches (user_id, created_at);
