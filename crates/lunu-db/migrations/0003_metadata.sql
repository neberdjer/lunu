CREATE TABLE metadata_cache (
	provider TEXT NOT NULL,
	kind TEXT NOT NULL,
	key TEXT NOT NULL,
	payload TEXT NOT NULL,
	fetched_at TEXT NOT NULL,
	PRIMARY KEY (provider, kind, key)
);
