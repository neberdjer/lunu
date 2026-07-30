CREATE TABLE notification_deliveries (
	job_id TEXT NOT NULL,
	channel TEXT NOT NULL,
	delivered_at TEXT NOT NULL,
	PRIMARY KEY (job_id, channel)
);
