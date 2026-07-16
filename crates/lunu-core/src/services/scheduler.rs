use std::str::FromStr;
use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::consts::jobs::DEFAULT_SCHEDULES;
use crate::consts::reasons;
use crate::models::{JobType, Schedule};
use crate::repo::ScheduleRepo;
use crate::services::JobService;
use crate::{Error, Result};

pub struct SchedulerService {
	schedules: Arc<dyn ScheduleRepo>,
	jobs: Arc<JobService>,
}

impl SchedulerService {
	pub fn new(schedules: Arc<dyn ScheduleRepo>, jobs: Arc<JobService>) -> Self {
		Self { schedules, jobs }
	}

	pub async fn ensure_defaults(&self) -> Result<()> {
		let now = Utc::now();
		for (kind, interval_secs) in DEFAULT_SCHEDULES {
			self.schedules
				.insert_if_absent(&Schedule {
					kind: kind.as_str().to_string(),
					interval_secs: *interval_secs,
					enabled: false,
					next_run_at: now,
					last_run_at: None,
					updated_at: now,
				})
				.await?;
		}
		Ok(())
	}

	pub async fn run_due(&self) -> Result<usize> {
		let now = Utc::now();
		let mut enqueued = 0;

		for schedule in self.schedules.due(now).await? {
			let Ok(job_type) = JobType::from_str(&schedule.kind) else {
				continue;
			};
			if self.jobs.enqueue_detached(job_type).await? {
				enqueued += 1;
			}
			let next = now + Duration::seconds(schedule.interval_secs);
			self.schedules.advance(&schedule.kind, now, next).await?;
		}

		Ok(enqueued)
	}

	pub async fn list(&self) -> Result<Vec<Schedule>> {
		self.schedules.list().await
	}

	pub async fn configure(&self, kind: &str, enabled: bool, interval_secs: i64) -> Result<bool> {
		const MAX_INTERVAL_SECS: i64 = 366 * 24 * 60 * 60;
		if !(1..=MAX_INTERVAL_SECS).contains(&interval_secs) {
			return Err(Error::Validation(
				reasons::SCHEDULE_INTERVAL_INVALID.to_string(),
			));
		}
		let next = Utc::now() + Duration::seconds(interval_secs);
		self.schedules
			.configure(kind, enabled, interval_secs, next)
			.await
	}
}
