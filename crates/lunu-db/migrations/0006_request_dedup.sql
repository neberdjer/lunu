CREATE UNIQUE INDEX idx_requests_active
	ON requests (user_id, asin)
	WHERE status NOT IN ('declined', 'failed');
