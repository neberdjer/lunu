INSERT INTO works (id, title, author, cover_url, created_at)
	SELECT
		'work-asin-' || r.asin,
		MIN(r.title),
		MIN(r.author),
		MIN(r.cover_url),
		MIN(r.created_at)
	FROM requests r
	WHERE r.asin IS NOT NULL
	GROUP BY r.asin;

INSERT INTO work_external_ids (scheme, value, work_id)
	SELECT DISTINCT 'asin', r.asin, 'work-asin-' || r.asin
	FROM requests r
	WHERE r.asin IS NOT NULL;

INSERT INTO works (id, title, author, cover_url, created_at)
	SELECT
		'work-request-' || r.id,
		r.title,
		r.author,
		r.cover_url,
		r.created_at
	FROM requests r
	WHERE r.asin IS NULL;

CREATE TABLE requests_new (
	id TEXT PRIMARY KEY,
	user_id TEXT NOT NULL,
	work_id TEXT NOT NULL,
	format TEXT NOT NULL DEFAULT 'audiobook',
	asin TEXT,
	title TEXT NOT NULL,
	author TEXT,
	cover_url TEXT,
	status TEXT NOT NULL,
	approved_by TEXT,
	created_at TEXT NOT NULL,
	updated_at TEXT NOT NULL,
	notes TEXT,
	quality_profile_id TEXT
);

INSERT INTO requests_new
	(id, user_id, work_id, format, asin, title, author, cover_url, status, approved_by, created_at, updated_at, notes, quality_profile_id)
	SELECT
		r.id,
		r.user_id,
		COALESCE(e.work_id, 'work-request-' || r.id),
		'audiobook',
		r.asin,
		r.title,
		r.author,
		r.cover_url,
		r.status,
		r.approved_by,
		r.created_at,
		r.updated_at,
		r.notes,
		r.quality_profile_id
	FROM requests r
	LEFT JOIN work_external_ids e ON e.scheme = 'asin' AND e.value = r.asin;

DROP TABLE requests;

ALTER TABLE requests_new RENAME TO requests;

CREATE INDEX idx_requests_user_id ON requests (user_id);
CREATE INDEX idx_requests_asin ON requests (asin);
CREATE INDEX idx_requests_user_asin ON requests (user_id, asin);
CREATE INDEX idx_requests_user_work ON requests (user_id, work_id);
CREATE UNIQUE INDEX idx_requests_active
	ON requests (user_id, work_id, format)
	WHERE status NOT IN ('declined', 'failed');
