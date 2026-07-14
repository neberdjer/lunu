CREATE TABLE requests_new (
	id TEXT PRIMARY KEY,
	user_id TEXT NOT NULL,
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
	(id, user_id, asin, title, author, cover_url, status, approved_by, created_at, updated_at, notes, quality_profile_id)
	SELECT id, user_id, asin, title, author, cover_url, status, approved_by, created_at, updated_at, notes, quality_profile_id
	FROM requests;

DROP TABLE requests;

ALTER TABLE requests_new RENAME TO requests;

CREATE INDEX idx_requests_user_id ON requests (user_id);
CREATE INDEX idx_requests_asin ON requests (asin);
CREATE UNIQUE INDEX idx_requests_active
	ON requests (user_id, asin)
	WHERE status NOT IN ('declined', 'failed');
