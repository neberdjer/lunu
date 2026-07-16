CREATE TABLE metadata_cache (
	provider TEXT NOT NULL,
	kind TEXT NOT NULL,
	key TEXT NOT NULL,
	payload TEXT NOT NULL,
	fetched_at TEXT NOT NULL,
	PRIMARY KEY (provider, kind, key)
);

CREATE TABLE works (
	id TEXT PRIMARY KEY,
	title TEXT NOT NULL,
	author TEXT,
	normalized_title TEXT,
	normalized_author TEXT,
	cover_url TEXT,
	created_at TEXT NOT NULL
);
CREATE INDEX idx_works_normalized ON works (normalized_title, normalized_author);

CREATE TABLE work_external_ids (
	scheme TEXT NOT NULL,
	value TEXT NOT NULL,
	work_id TEXT NOT NULL,
	PRIMARY KEY (scheme, value)
);
CREATE INDEX idx_work_external_ids_work ON work_external_ids (work_id);
