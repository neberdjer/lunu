use chrono::Duration;
use lunu_core::models::{BlocklistEntry, Session};
use lunu_core::repo::{BlocklistRepo, SessionRepo};

use super::builders::*;
use super::*;

fn job_row(id: &str) -> Job {
	let now = Utc::now();
	Job {
		id: id.to_string(),
		job_type: JobType::Grab,
		request_id: None,
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

fn session_row(id: &str, user_id: &str, created_at: chrono::DateTime<Utc>) -> Session {
	Session {
		id: id.to_string(),
		user_id: user_id.to_string(),
		token_hash: hash_token(id),
		user_agent: None,
		created_at,
		expires_at: created_at + Duration::days(30),
		last_seen_at: None,
	}
}

#[tokio::test]
async fn pruning_removes_only_old_finished_jobs() {
	let db = memory_db().await;
	let repo = SqlxJobRepo::new(db.clone());
	let now = Utc::now();
	let old = now - Duration::days(30);

	for (id, status) in [
		("old-done", JobStatus::Completed),
		("old-failed", JobStatus::Failed),
		("old-pending", JobStatus::Pending),
	] {
		let mut job = job_row(id);
		job.status = status;
		job.updated_at = old;
		repo.create(&job).await.unwrap();
	}
	let mut fresh = job_row("fresh-done");
	fresh.status = JobStatus::Completed;
	fresh.updated_at = now;
	repo.create(&fresh).await.unwrap();

	let pruned = repo
		.delete_finished_before(now - Duration::days(14))
		.await
		.unwrap();
	assert_eq!(pruned, 2, "only the old terminal jobs are pruned");

	assert!(repo.find_by_id("old-done").await.unwrap().is_none());
	assert!(repo.find_by_id("old-failed").await.unwrap().is_none());
	assert!(
		repo.find_by_id("old-pending").await.unwrap().is_some(),
		"pending work must never be pruned no matter how old"
	);
	assert!(
		repo.find_by_id("fresh-done").await.unwrap().is_some(),
		"recent history stays inside the retention window"
	);
}

#[tokio::test]
async fn renew_lease_only_for_the_holding_worker() {
	let db = memory_db().await;
	let repo = SqlxJobRepo::new(db.clone());
	repo.create(&job_row("j-lease")).await.unwrap();

	let claimed = repo.claim_next("worker-a", Utc::now()).await.unwrap();
	let claimed = claimed.expect("job is claimable");
	let before = claimed.locked_at.expect("claim sets locked_at");

	let later = Utc::now() + Duration::seconds(120);
	assert!(
		repo.renew_lease(&claimed.id, "worker-a", later)
			.await
			.unwrap()
	);
	assert!(
		!repo
			.renew_lease(&claimed.id, "worker-b", later)
			.await
			.unwrap()
	);

	let after = repo
		.find_by_id(&claimed.id)
		.await
		.unwrap()
		.unwrap()
		.locked_at
		.unwrap();
	assert!(after > before);
}

#[tokio::test]
async fn renewed_lease_survives_the_reaper() {
	let db = memory_db().await;
	let repo = SqlxJobRepo::new(db.clone());
	repo.create(&job_row("j-reap")).await.unwrap();
	let claimed = repo
		.claim_next("worker-a", Utc::now())
		.await
		.unwrap()
		.unwrap();

	let now = Utc::now();
	repo.renew_lease(&claimed.id, "worker-a", now)
		.await
		.unwrap();

	let reaped = repo
		.reap_stale(now - Duration::seconds(300), now)
		.await
		.unwrap();
	assert_eq!(reaped, 0);
	assert_eq!(
		repo.find_by_id(&claimed.id).await.unwrap().unwrap().status,
		JobStatus::Running
	);
}

#[tokio::test]
async fn sessions_page_and_count_scope_to_the_user() {
	let db = memory_db().await;
	let users = SqlxUserRepo::new(db.clone());
	let repo = SqlxSessionRepo::new(db.clone());

	for (id, name) in [("u-1", "alice"), ("u-2", "bob")] {
		let now = Utc::now();
		users
			.create(&User {
				id: id.to_string(),
				username: name.to_string(),
				email: None,
				display_name: None,
				locale: None,
				password_hash: None,
				role: Role::User,
				auth_source: AuthSource::Local,
				oidc_subject: None,
				enabled: true,
				email_verified: false,
				created_at: now,
				updated_at: now,
			})
			.await
			.unwrap();
	}

	let base = Utc::now();
	for i in 0..3 {
		repo.create(&session_row(
			&format!("s-a{i}"),
			"u-1",
			base + Duration::seconds(i),
		))
		.await
		.unwrap();
	}
	repo.create(&session_row("s-b0", "u-2", base))
		.await
		.unwrap();

	assert_eq!(repo.count_for_user("u-1").await.unwrap(), 3);
	assert_eq!(repo.count_for_user("u-2").await.unwrap(), 1);

	let page = repo.list_for_user_page("u-1", 2, 0).await.unwrap();
	assert_eq!(page.len(), 2);
	assert!(page.iter().all(|s| s.user_id == "u-1"));

	let next = repo.list_for_user_page("u-1", 2, 2).await.unwrap();
	assert_eq!(next.len(), 1);
}

#[tokio::test]
async fn blocklist_remove_by_id_is_scoped_to_the_request() {
	let db = memory_db().await;
	let repo = SqlxBlocklistRepo::new(db.clone());

	let entry = BlocklistEntry {
		id: "b-1".to_string(),
		request_id: "r-1".to_string(),
		download_url: "magnet:?xt=urn:btih:aaa".to_string(),
		created_at: Utc::now(),
	};
	repo.add(&entry).await.unwrap();

	assert!(!repo.remove_by_id("r-other", "b-1").await.unwrap());
	assert!(!repo.remove_by_id("r-1", "b-missing").await.unwrap());
	assert_eq!(repo.list_for_request("r-1").await.unwrap().len(), 1);

	assert!(repo.remove_by_id("r-1", "b-1").await.unwrap());
	assert!(repo.list_for_request("r-1").await.unwrap().is_empty());
}

#[tokio::test]
async fn job_claim_is_atomic_and_lifecycle_transitions() {
	let db = memory_db().await;
	let repo = SqlxJobRepo::new(db.clone());

	let now = Utc::now();
	repo.create(&pending_job("j1", now)).await.unwrap();

	let claimed = repo
		.claim_next("worker-a", Utc::now())
		.await
		.unwrap()
		.unwrap();
	assert_eq!(claimed.id, "j1");
	assert_eq!(claimed.status, JobStatus::Running);
	assert_eq!(claimed.attempts, 1);
	assert_eq!(claimed.locked_by.as_deref(), Some("worker-a"));

	assert!(
		repo.claim_next("worker-b", Utc::now())
			.await
			.unwrap()
			.is_none()
	);

	let future = Utc::now() + chrono::Duration::seconds(30);
	repo.reschedule("j1", "worker-a", "temporary", future, Utc::now(), 5)
		.await
		.unwrap();
	let after = repo.find_by_id("j1").await.unwrap().unwrap();
	assert_eq!(after.status, JobStatus::Pending);
	assert_eq!(after.attempts, 1);
	assert_eq!(after.last_error.as_deref(), Some("temporary"));
	assert!(after.locked_by.is_none());

	assert!(
		repo.claim_next("worker-a", Utc::now())
			.await
			.unwrap()
			.is_none()
	);

	let reclaimed = repo
		.claim_next("worker-a", future + chrono::Duration::seconds(1))
		.await
		.unwrap()
		.unwrap();
	assert_eq!(reclaimed.attempts, 2);

	repo.complete("j1", "worker-a", Utc::now()).await.unwrap();
	assert_eq!(
		repo.find_by_id("j1").await.unwrap().unwrap().status,
		JobStatus::Completed
	);
	assert_eq!(repo.list().await.unwrap().len(), 1);
}

#[tokio::test]
async fn reap_stale_returns_running_jobs_to_pending() {
	let db = memory_db().await;
	let repo = SqlxJobRepo::new(db.clone());

	let now = Utc::now();
	repo.create(&pending_job("j2", now)).await.unwrap();
	repo.claim_next("worker-a", now).await.unwrap().unwrap();

	assert_eq!(
		repo.reap_stale(now - chrono::Duration::seconds(300), Utc::now())
			.await
			.unwrap(),
		0
	);

	let reaped = repo
		.reap_stale(now + chrono::Duration::seconds(1), Utc::now())
		.await
		.unwrap();
	assert_eq!(reaped, 1);

	let after = repo.find_by_id("j2").await.unwrap().unwrap();
	assert_eq!(after.status, JobStatus::Pending);
	assert!(after.locked_by.is_none());
	assert_eq!(after.attempts, 1);
}
