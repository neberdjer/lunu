CREATE TABLE quality_profiles (
	id TEXT PRIMARY KEY,
	name TEXT NOT NULL,
	allowed_formats TEXT NOT NULL,
	preferred_formats TEXT NOT NULL,
	min_seeders BIGINT NOT NULL DEFAULT 1,
	min_size_mb BIGINT,
	max_size_mb BIGINT,
	seeder_weight BIGINT NOT NULL DEFAULT 1,
	format_weight BIGINT NOT NULL DEFAULT 100,
	preferred_keywords TEXT NOT NULL DEFAULT '',
	avoided_keywords TEXT NOT NULL DEFAULT '',
	keyword_weight BIGINT NOT NULL DEFAULT 100,
	is_default BIGINT NOT NULL DEFAULT 0,
	created_at TEXT NOT NULL,
	updated_at TEXT NOT NULL
);

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
	updated_at TEXT NOT NULL,
	info_hash TEXT,
	progress BIGINT NOT NULL DEFAULT 0
);
CREATE INDEX idx_downloads_request_id ON downloads (request_id);

CREATE TABLE blocklist (
	id TEXT PRIMARY KEY,
	request_id TEXT NOT NULL,
	download_url TEXT NOT NULL,
	created_at TEXT NOT NULL
);
CREATE UNIQUE INDEX idx_blocklist_request_url ON blocklist (request_id, download_url);
