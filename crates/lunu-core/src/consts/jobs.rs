use crate::models::JobType;

pub const DEFAULT_MAX_ATTEMPTS: i64 = 5;
pub const TRANSIENT_MAX_ATTEMPTS: i64 = 100;
pub const RETRY_BASE_SECS: i64 = 15;
pub const RETRY_MAX_SECS: i64 = 3600;
pub const LEASE_TIMEOUT_SECS: i64 = 300;
pub const LEASE_RENEW_SECS: u64 = 60;
pub const MAX_JOB_SECS: u64 = 3600;
pub const POLL_INTERVAL_MS: u64 = 1000;
pub const DEFAULT_WORKER_COUNT: usize = 2;

pub const SCHEDULER_TICK_SECS: u64 = 60;
pub const LIBRARY_SYNC_INTERVAL_SECS: i64 = 6 * 60 * 60;
pub const SESSION_CLEANUP_INTERVAL_SECS: i64 = 24 * 60 * 60;
pub const JOB_CLEANUP_INTERVAL_SECS: i64 = 24 * 60 * 60;
pub const JOB_RETENTION_DAYS: i64 = 14;

pub const DEFAULT_SCHEDULES: &[(JobType, i64)] = &[
	(JobType::LibrarySync, LIBRARY_SYNC_INTERVAL_SECS),
	(JobType::SessionCleanup, SESSION_CLEANUP_INTERVAL_SECS),
	(JobType::JobCleanup, JOB_CLEANUP_INTERVAL_SECS),
];
