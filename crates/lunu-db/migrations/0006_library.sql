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
	merged_path TEXT,
	merge_state TEXT NOT NULL DEFAULT 'idle',
	merge_detail TEXT,
	merge_backup_path TEXT,
	source TEXT NOT NULL DEFAULT 'request',
	overridden BIGINT NOT NULL DEFAULT 0,
	matched_by TEXT,
	request_id TEXT,
	created_at TEXT NOT NULL
);
CREATE INDEX idx_media_merge_state ON media (merge_state);
CREATE INDEX idx_media_created_at ON media (created_at);
