use std::sync::{Arc, OnceLock};

use std::time::Duration;
use tokio::runtime::{Handle, Runtime};

use chrono::Utc;
use lunu_core::consts::jobs::{
	DEFAULT_MEDIA_WORKER_COUNT, DEFAULT_WORKER_COUNT, LEASE_RENEW_SECS, LEASE_TIMEOUT_SECS,
	MAX_JOB_SECS, MIN_JOB_RUNTIME_THREADS, POLL_INTERVAL_MS, SCHEDULER_TICK_SECS,
	TRANSIENT_MAX_ATTEMPTS,
};
use lunu_core::models::{Job, JobType};
use lunu_core::repo::JobRepo;
use lunu_core::services::SchedulerService;
use lunu_core::traits::JobHandler;
use lunu_core::{Error, Result};

mod pipeline;

pub use pipeline::PipelineHandler;

#[derive(Debug, Clone, Copy)]
pub struct WorkerConfig {
	pub workers: usize,
	pub media_workers: usize,
	pub poll_interval: Duration,
	pub lease_timeout: Duration,
}

impl Default for WorkerConfig {
	fn default() -> Self {
		Self {
			workers: DEFAULT_WORKER_COUNT,
			media_workers: DEFAULT_MEDIA_WORKER_COUNT,
			poll_interval: Duration::from_millis(POLL_INTERVAL_MS),
			lease_timeout: Duration::from_secs(LEASE_TIMEOUT_SECS as u64),
		}
	}
}

static JOB_RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub fn job_runtime(threads: usize) -> std::io::Result<&'static Handle> {
	if JOB_RUNTIME.get().is_none() {
		let runtime = tokio::runtime::Builder::new_multi_thread()
			.worker_threads(threads.max(MIN_JOB_RUNTIME_THREADS))
			.thread_name("lunu-job")
			.enable_all()
			.build()?;
		let _ = JOB_RUNTIME.set(runtime);
	}
	Ok(JOB_RUNTIME
		.get()
		.expect("job runtime was just initialised")
		.handle())
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

	pub fn start(self, runtime: &Handle) {
		let lanes = [
			("worker", self.config.workers, JobType::general_lane()),
			("media", self.config.media_workers, JobType::media_lane()),
		];
		for (name, count, lane) in lanes {
			for index in 0..count {
				let jobs = self.jobs.clone();
				let handler = self.handler.clone();
				let poll_interval = self.config.poll_interval;
				let worker_id = format!("{name}-{index}");
				let lane = lane.clone();
				runtime.spawn(worker_loop(worker_id, jobs, handler, poll_interval, lane));
			}
		}

		let jobs = self.jobs.clone();
		let lease_timeout = self.config.lease_timeout;
		runtime.spawn(reaper_loop(jobs, lease_timeout));
	}
}

async fn worker_loop(
	worker_id: String,
	jobs: Arc<dyn JobRepo>,
	handler: Arc<dyn JobHandler>,
	poll_interval: Duration,
	lane: Vec<JobType>,
) {
	loop {
		match jobs.claim_next(&worker_id, Utc::now(), &lane).await {
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
	let mut work = {
		let handler = handler.clone();
		let job = job.clone();
		AbortOnDrop(tokio::spawn(async move { handler.handle(&job).await }))
	};
	let heartbeat = {
		let jobs = jobs.clone();
		let id = job.id.clone();
		let locked_by = job.locked_by.clone().unwrap_or_default();
		AbortOnDrop(tokio::spawn(async move {
			loop {
				tokio::time::sleep(Duration::from_secs(LEASE_RENEW_SECS)).await;
				if let Ok(false) | Err(_) = jobs.renew_lease(&id, &locked_by, Utc::now()).await {
					return;
				}
			}
		}))
	};

	let outcome = match tokio::time::timeout(Duration::from_secs(MAX_JOB_SECS), &mut work.0).await {
		Ok(Ok(result)) => result,
		Ok(Err(join)) if join.is_panic() => {
			Err(Error::Internal("job handler panicked".to_string()))
		}
		Ok(Err(_)) => Err(Error::Internal("job handler was cancelled".to_string())),
		Err(_) => Err(Error::Internal(format!(
			"job exceeded maximum duration of {MAX_JOB_SECS}s"
		))),
	};

	drop(work);
	drop(heartbeat);
	outcome
}

struct AbortOnDrop<T>(tokio::task::JoinHandle<T>);

impl<T> Drop for AbortOnDrop<T> {
	fn drop(&mut self) {
		self.0.abort();
	}
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

	pub fn start(self, runtime: &Handle) {
		runtime.spawn(scheduler_loop(self.scheduler, self.tick));
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
