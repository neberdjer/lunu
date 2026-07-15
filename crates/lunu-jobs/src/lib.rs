use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use lunu_core::consts::jobs::{
	DEFAULT_WORKER_COUNT, LEASE_RENEW_SECS, LEASE_TIMEOUT_SECS, MAX_JOB_SECS, POLL_INTERVAL_MS,
	SCHEDULER_TICK_SECS, TRANSIENT_MAX_ATTEMPTS,
};
use lunu_core::models::Job;
use lunu_core::repo::JobRepo;
use lunu_core::services::SchedulerService;
use lunu_core::traits::JobHandler;
use lunu_core::{Error, Result};

mod pipeline;

pub use pipeline::PipelineHandler;

#[derive(Debug, Clone, Copy)]
pub struct WorkerConfig {
	pub workers: usize,
	pub poll_interval: Duration,
	pub lease_timeout: Duration,
}

impl Default for WorkerConfig {
	fn default() -> Self {
		Self {
			workers: DEFAULT_WORKER_COUNT,
			poll_interval: Duration::from_millis(POLL_INTERVAL_MS),
			lease_timeout: Duration::from_secs(LEASE_TIMEOUT_SECS as u64),
		}
	}
}

pub struct WorkerPool {
	jobs: Arc<dyn JobRepo>,
	handler: Arc<dyn JobHandler>,
	config: WorkerConfig,
}

impl WorkerPool {
	pub fn new(jobs: Arc<dyn JobRepo>, handler: Arc<dyn JobHandler>, config: WorkerConfig) -> Self {
		Self {
			jobs,
			handler,
			config,
		}
	}

	pub fn start(self) {
		for index in 0..self.config.workers {
			let jobs = self.jobs.clone();
			let handler = self.handler.clone();
			let poll_interval = self.config.poll_interval;
			let worker_id = format!("worker-{index}");
			tokio::spawn(worker_loop(worker_id, jobs, handler, poll_interval));
		}

		let jobs = self.jobs.clone();
		let lease_timeout = self.config.lease_timeout;
		tokio::spawn(reaper_loop(jobs, lease_timeout));
	}
}

async fn worker_loop(
	worker_id: String,
	jobs: Arc<dyn JobRepo>,
	handler: Arc<dyn JobHandler>,
	poll_interval: Duration,
) {
	loop {
		match jobs.claim_next(&worker_id, Utc::now()).await {
			Ok(Some(job)) => run_job(&jobs, &handler, job).await,
			Ok(None) => tokio::time::sleep(poll_interval).await,
			Err(error) => {
				tracing::error!(%error, worker = %worker_id, "failed to claim job");
				tokio::time::sleep(poll_interval).await;
			}
		}
	}
}

async fn run_job(jobs: &Arc<dyn JobRepo>, handler: &Arc<dyn JobHandler>, mut job: Job) {
	let outcome = run_with_lease(jobs, handler, &job).await;
	let now = Utc::now();
	let locked_by = job.locked_by.clone().unwrap_or_default();
	let locked_by = locked_by.as_str();

	let result = match outcome {
		Ok(()) => jobs.complete(&job.id, locked_by, now).await,
		Err(error) => {
			if error.is_transient() {
				job.max_attempts = job.max_attempts.max(TRANSIENT_MAX_ATTEMPTS);
			}
			let error = error.to_string();
			if job.should_retry() {
				let run_after = now + job.retry_backoff();
				tracing::warn!(job = %job.id, kind = %job.job_type, attempt = job.attempts, %error, "job failed, retrying");
				jobs.reschedule(&job.id, locked_by, &error, run_after, now, job.max_attempts)
					.await
			} else {
				tracing::error!(job = %job.id, kind = %job.job_type, %error, "job failed permanently");
				let result = jobs.fail(&job.id, locked_by, &error, now).await;
				handler.on_failed(&job, &error).await;
				result
			}
		}
	};

	if let Err(error) = result {
		tracing::error!(job = %job.id, %error, "failed to record job outcome");
	}
}

async fn run_with_lease(
	jobs: &Arc<dyn JobRepo>,
	handler: &Arc<dyn JobHandler>,
	job: &Job,
) -> Result<()> {
	let heartbeat = {
		let jobs = jobs.clone();
		let id = job.id.clone();
		let locked_by = job.locked_by.clone().unwrap_or_default();
		tokio::spawn(async move {
			loop {
				tokio::time::sleep(Duration::from_secs(LEASE_RENEW_SECS)).await;
				if let Ok(false) | Err(_) = jobs.renew_lease(&id, &locked_by, Utc::now()).await {
					return;
				}
			}
		})
	};

	let outcome =
		match tokio::time::timeout(Duration::from_secs(MAX_JOB_SECS), handler.handle(job)).await {
			Ok(result) => result,
			Err(_) => Err(Error::Internal(format!(
				"job exceeded maximum duration of {MAX_JOB_SECS}s"
			))),
		};

	heartbeat.abort();
	outcome
}

async fn reaper_loop(jobs: Arc<dyn JobRepo>, lease_timeout: Duration) {
	let lease = chrono::Duration::seconds(lease_timeout.as_secs() as i64);
	loop {
		tokio::time::sleep(lease_timeout).await;
		let now = Utc::now();
		match jobs.reap_stale(now - lease, now).await {
			Ok(0) => {}
			Ok(reaped) => tracing::warn!(reaped, "reclaimed stale jobs"),
			Err(error) => tracing::error!(%error, "failed to reap stale jobs"),
		}
	}
}

pub struct SchedulerPool {
	scheduler: Arc<SchedulerService>,
	tick: Duration,
}

impl SchedulerPool {
	pub fn new(scheduler: Arc<SchedulerService>) -> Self {
		Self {
			scheduler,
			tick: Duration::from_secs(SCHEDULER_TICK_SECS),
		}
	}

	pub fn start(self) {
		tokio::spawn(scheduler_loop(self.scheduler, self.tick));
	}
}

async fn scheduler_loop(scheduler: Arc<SchedulerService>, tick: Duration) {
	loop {
		tokio::time::sleep(tick).await;
		match scheduler.run_due().await {
			Ok(0) => {}
			Ok(enqueued) => tracing::info!(enqueued, "scheduler enqueued due jobs"),
			Err(error) => tracing::error!(%error, "scheduler tick failed"),
		}
	}
}

#[cfg(test)]
mod tests;
