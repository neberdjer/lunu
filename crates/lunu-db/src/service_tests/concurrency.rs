use std::collections::HashSet;

use chrono::Duration;

use super::*;

const WORKERS: usize = 8;
const JOB_COUNT: usize = 60;

fn pending_job(id: &str) -> Job {
	let now = Utc::now();
	Job {
		id: id.to_string(),
		job_type: JobType::Grab,
		request_id: Some(format!("req-{id}")),
		payload: "{}".to_string(),
		status: JobStatus::Pending,
		attempts: 0,
		max_attempts: 5,
		run_after: now,
		locked_by: None,
		locked_at: None,
		last_error: None,
		created_at: now,
		updated_at: now,
	}
}

async fn seed(repo: &SqlxJobRepo, count: usize) {
	for i in 0..count {
		repo.create(&pending_job(&format!("job-{i:03}")))
			.await
			.unwrap();
	}
}

async fn pg_pool(connections: u32) -> Option<Db> {
	let db = concurrent_db(connections).await;
	if db.is_none() {
		eprintln!(
			"skipped: concurrency requires a real pool; set LUNU_TEST_DATABASE_URL to a postgres url"
		);
	}
	db
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn concurrent_workers_claim_each_job_exactly_once() {
	let Some(db) = pg_pool(WORKERS as u32).await else {
		return;
	};
	let repo = Arc::new(SqlxJobRepo::new(db.clone()));
	seed(&repo, JOB_COUNT).await;

	let mut handles = Vec::new();
	for worker in 0..WORKERS {
		let repo = repo.clone();
		handles.push(tokio::spawn(async move {
			let worker_id = format!("worker-{worker}");
			let mut claimed = Vec::new();
			while let Some(job) = repo.claim_next(&worker_id, Utc::now()).await.unwrap() {
				claimed.push(job.id);
			}
			claimed
		}));
	}

	let mut all = Vec::new();
	for handle in handles {
		all.extend(handle.await.unwrap());
	}

	let unique: HashSet<&String> = all.iter().collect();
	assert_eq!(all.len(), JOB_COUNT, "every job is claimed");
	assert_eq!(unique.len(), JOB_COUNT, "no job is claimed twice");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn concurrent_claim_increments_attempts_once_per_job() {
	let Some(db) = pg_pool(WORKERS as u32).await else {
		return;
	};
	let repo = Arc::new(SqlxJobRepo::new(db.clone()));
	seed(&repo, JOB_COUNT).await;

	let mut handles = Vec::new();
	for worker in 0..WORKERS {
		let repo = repo.clone();
		handles.push(tokio::spawn(async move {
			let worker_id = format!("worker-{worker}");
			while repo
				.claim_next(&worker_id, Utc::now())
				.await
				.unwrap()
				.is_some()
			{}
		}));
	}
	for handle in handles {
		handle.await.unwrap();
	}

	for i in 0..JOB_COUNT {
		let job = repo
			.find_by_id(&format!("job-{i:03}"))
			.await
			.unwrap()
			.expect("job exists");
		assert_eq!(job.attempts, 1, "claim increments attempts exactly once");
		assert_eq!(job.status, JobStatus::Running);
		assert!(job.locked_by.is_some());
	}
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn only_the_lease_holder_can_finalize_a_job() {
	let Some(db) = pg_pool(4).await else {
		return;
	};
	let repo = Arc::new(SqlxJobRepo::new(db.clone()));
	repo.create(&pending_job("job-fence")).await.unwrap();
	let claimed = repo
		.claim_next("holder", Utc::now())
		.await
		.unwrap()
		.expect("claimable");

	let now = Utc::now();
	repo.complete(&claimed.id, "impostor", now).await.unwrap();
	assert_eq!(
		repo.find_by_id(&claimed.id).await.unwrap().unwrap().status,
		JobStatus::Running,
		"a non-holder cannot complete the job"
	);

	repo.complete(&claimed.id, "holder", now).await.unwrap();
	assert_eq!(
		repo.find_by_id(&claimed.id).await.unwrap().unwrap().status,
		JobStatus::Completed
	);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn reaper_reclaims_only_expired_leases_and_allows_reclaim() {
	let Some(db) = pg_pool(4).await else {
		return;
	};
	let repo = Arc::new(SqlxJobRepo::new(db.clone()));
	repo.create(&pending_job("job-stale")).await.unwrap();
	let claimed = repo
		.claim_next("dead-worker", Utc::now())
		.await
		.unwrap()
		.expect("claimable");

	let now = Utc::now();
	assert_eq!(
		repo.reap_stale(now - Duration::seconds(300), now)
			.await
			.unwrap(),
		0,
		"a fresh lease is not reaped"
	);

	let future = now + Duration::seconds(600);
	assert_eq!(
		repo.reap_stale(future - Duration::seconds(300), future)
			.await
			.unwrap(),
		1,
		"an expired lease is reaped"
	);

	let reclaimed = repo
		.claim_next("live-worker", future)
		.await
		.unwrap()
		.expect("reaped job is claimable again");
	assert_eq!(reclaimed.id, claimed.id);
	assert_eq!(reclaimed.attempts, 2, "reclaim counts as another attempt");
}
