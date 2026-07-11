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
	is_default BIGINT NOT NULL DEFAULT 0,
	created_at TEXT NOT NULL,
	updated_at TEXT NOT NULL
);
