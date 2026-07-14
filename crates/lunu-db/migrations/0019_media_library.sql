CREATE TABLE media_new (
	id TEXT PRIMARY KEY,
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

INSERT INTO media_new
	(id, asin, title, author, cover_url, library_path, source, request_id, created_at)
	SELECT asin, asin, title, author, cover_url, library_path, 'request', request_id, created_at
	FROM media;

DROP TABLE media;

ALTER TABLE media_new RENAME TO media;
