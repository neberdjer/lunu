use super::*;

fn job(attempts: i64, max_attempts: i64) -> Job {
	let now = Utc::now();
	Job {
		id: "j1".to_string(),
		job_type: JobType::Grab,
		request_id: None,
		dedupe_key: None,
		payload: "{}".to_string(),
		status: JobStatus::Running,
		attempts,
		max_attempts,
		run_after: now,
		locked_by: None,
		locked_at: None,
		last_error: None,
		created_at: now,
		updated_at: now,
	}
}

#[test]
fn only_fulfillment_jobs_fail_the_users_request() {
	for job_type in [JobType::Grab, JobType::MonitorDownload, JobType::Import] {
		assert!(
			job_type.propagates_failure_to_request(),
			"{job_type} is part of fulfilling a request, so exhausting it must fail the request"
		);
	}
	for job_type in [
		JobType::Merge,
		JobType::Notify,
		JobType::LibrarySync,
		JobType::SessionCleanup,
		JobType::JobCleanup,
	] {
		assert!(
			!job_type.propagates_failure_to_request(),
			"{job_type} is background work, so failing it must never fail a user's request"
		);
	}
}

#[test]
fn job_type_round_trips_through_its_wire_name() {
	for job_type in [
		JobType::Grab,
		JobType::MonitorDownload,
		JobType::Import,
		JobType::Merge,
		JobType::Notify,
		JobType::LibrarySync,
		JobType::SessionCleanup,
		JobType::JobCleanup,
	] {
		assert_eq!(JobType::from_str(job_type.as_str()).unwrap(), job_type);
	}
}

#[test]
fn the_scheduled_jobs_are_exactly_the_recurring_ones() {
	use crate::consts::jobs::DEFAULT_SCHEDULES;

	let scheduled: Vec<JobType> = DEFAULT_SCHEDULES.iter().map(|(kind, _)| *kind).collect();
	for job_type in &scheduled {
		assert!(
			job_type.is_recurring(),
			"{job_type} is on a schedule, so it must be treated as recurring"
		);
	}
	for job_type in [
		JobType::Grab,
		JobType::MonitorDownload,
		JobType::Import,
		JobType::Merge,
		JobType::Notify,
	] {
		assert!(
			!job_type.is_recurring(),
			"{job_type} is queued on demand, so a one-per-type index must not cap it"
		);
	}
}

#[test]
fn the_dedupe_index_covers_every_recurring_job() {
	let migration = include_str!("../../../../lunu-db/migrations/0007_jobs_and_schedules.sql");
	let index = migration
		.split("CREATE UNIQUE INDEX idx_jobs_active_recurring")
		.nth(1)
		.and_then(|rest| rest.split(';').next())
		.expect("the jobs migration declares the recurring dedupe index");
	for job_type in [
		JobType::LibrarySync,
		JobType::SessionCleanup,
		JobType::JobCleanup,
		JobType::Grab,
		JobType::Merge,
	] {
		assert_eq!(
			index.contains(&format!("'{}'", job_type.as_str())),
			job_type.is_recurring(),
			"the partial unique index and is_recurring disagree about {job_type}, so adding a \
			 scheduled job would silently queue duplicates of it"
		);
	}
}

#[test]
fn should_retry_until_max_attempts() {
	assert!(job(1, 3).should_retry());
	assert!(job(2, 3).should_retry());
	assert!(!job(3, 3).should_retry());
	assert!(!job(4, 3).should_retry());
}

#[test]
fn retry_backoff_grows_then_caps() {
	assert_eq!(
		job(1, 5).retry_backoff(),
		Duration::seconds(RETRY_BASE_SECS)
	);
	assert_eq!(
		job(2, 5).retry_backoff(),
		Duration::seconds(RETRY_BASE_SECS * 2)
	);
	assert_eq!(
		job(3, 5).retry_backoff(),
		Duration::seconds(RETRY_BASE_SECS * 4)
	);
	assert!(job(40, 50).retry_backoff() <= Duration::seconds(RETRY_MAX_SECS));
}

#[test]
fn every_job_that_acts_on_a_media_row_reports_itself_as_one() {
	for job_type in [JobType::Merge, JobType::MergeRevert] {
		assert!(
			job_type.media_subject(),
			"{job_type} carries a media id, so a failure must be able to reach that row"
		);
	}
	for job_type in [
		JobType::Grab,
		JobType::MonitorDownload,
		JobType::Import,
		JobType::Notify,
		JobType::LibrarySync,
		JobType::SessionCleanup,
		JobType::JobCleanup,
	] {
		assert!(
			!job_type.media_subject(),
			"{job_type} has no media id, so treating it as one would misreport the failure"
		);
	}
}

#[test]
fn a_media_job_is_never_also_a_recurring_one() {
	for job_type in [JobType::Merge, JobType::MergeRevert] {
		assert!(
			!job_type.is_recurring(),
			"{job_type} would otherwise be logged as a recurring job when it exhausts retries"
		);
	}
}
