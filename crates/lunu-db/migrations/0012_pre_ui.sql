ALTER TABLE activity ADD COLUMN actor TEXT;
ALTER TABLE requests ADD COLUMN notes TEXT;
ALTER TABLE requests ADD COLUMN quality_profile_id TEXT;
ALTER TABLE jobs ADD COLUMN request_id TEXT;
ALTER TABLE users ADD COLUMN display_name TEXT;

CREATE INDEX idx_jobs_request_id ON jobs (request_id);
