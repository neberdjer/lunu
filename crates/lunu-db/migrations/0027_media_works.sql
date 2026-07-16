INSERT INTO works (id, title, author, cover_url, created_at)
	SELECT
		'work-asin-' || m.asin,
		MIN(m.title),
		MIN(m.author),
		MIN(m.cover_url),
		MIN(m.created_at)
	FROM media m
	WHERE m.asin IS NOT NULL
		AND NOT EXISTS (
			SELECT 1 FROM work_external_ids e WHERE e.scheme = 'asin' AND e.value = m.asin
		)
	GROUP BY m.asin;

INSERT INTO work_external_ids (scheme, value, work_id)
	SELECT DISTINCT 'asin', m.asin, 'work-asin-' || m.asin
	FROM media m
	WHERE m.asin IS NOT NULL
		AND NOT EXISTS (
			SELECT 1 FROM work_external_ids e WHERE e.scheme = 'asin' AND e.value = m.asin
		);

CREATE TABLE media_new (
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

INSERT INTO media_new
	(id, work_id, format, asin, abs_item_id, title, author, cover_url, series_name, series_sequence, library_path, source, overridden, request_id, created_at)
	SELECT
		m.id,
		e.work_id,
		'audiobook',
		m.asin,
		m.abs_item_id,
		m.title,
		m.author,
		m.cover_url,
		m.series_name,
		m.series_sequence,
		m.library_path,
		m.source,
		m.overridden,
		m.request_id,
		m.created_at
	FROM media m
	LEFT JOIN work_external_ids e ON e.scheme = 'asin' AND e.value = m.asin;

DROP TABLE media;

ALTER TABLE media_new RENAME TO media;

