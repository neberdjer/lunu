CREATE UNIQUE INDEX idx_jobs_active_recurring
	ON jobs (job_type)
	WHERE request_id IS NULL AND status IN ('pending', 'running');
