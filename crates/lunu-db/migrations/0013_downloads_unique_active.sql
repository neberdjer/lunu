CREATE UNIQUE INDEX idx_downloads_active_request ON downloads (request_id) WHERE state != 'failed';
