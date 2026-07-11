use async_trait::async_trait;
use chrono::{DateTime, Utc};
use lunu_core::Result;
use lunu_core::models::{Job, JobStatus, JobType};
use lunu_core::repo::JobRepo;
use sqlx::Row;
use sqlx::any::AnyRow;

use super::{map_row_opt, map_rows};
use crate::convert::{format_dt, parse_dt, parse_dt_opt, parse_enum};
use crate::{Db, db_error};

pub struct SqlxJobRepo {
	db: Db,
}

impl SqlxJobRepo {
	pub fn new(db: Db) -> Self {
		Self { db }
	}
}

fn map_job(row: &AnyRow) -> Result<Job> {
	let job_type: String = row.try_get("job_type").map_err(db_error)?;
	let status: String = row.try_get("status").map_err(db_error)?;
	let run_after: String = row.try_get("run_after").map_err(db_error)?;
	let locked_at: Option<String> = row.try_get("locked_at").map_err(db_error)?;
	let created_at: String = row.try_get("created_at").map_err(db_error)?;
	let updated_at: String = row.try_get("updated_at").map_err(db_error)?;

	Ok(Job {
		id: row.try_get("id").map_err(db_error)?,
		job_type: parse_enum::<JobType>(&job_type)?,
		payload: row.try_get("payload").map_err(db_error)?,
		status: parse_enum::<JobStatus>(&status)?,
		attempts: row.try_get("attempts").map_err(db_error)?,
		max_attempts: row.try_get("max_attempts").map_err(db_error)?,
		run_after: parse_dt(&run_after)?,
		locked_by: row.try_get("locked_by").map_err(db_error)?,
		locked_at: parse_dt_opt(locked_at)?,
		last_error: row.try_get("last_error").map_err(db_error)?,
		created_at: parse_dt(&created_at)?,
		updated_at: parse_dt(&updated_at)?,
	})
}

#[async_trait]
impl JobRepo for SqlxJobRepo {
	async fn create(&self, job: &Job) -> Result<()> {
		sqlx::query(
			"INSERT INTO jobs \
			 (id, job_type, payload, status, attempts, max_attempts, run_after, locked_by, locked_at, last_error, created_at, updated_at) \
			 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
		)
		.bind(&job.id)
		.bind(job.job_type.as_str())
		.bind(&job.payload)
		.bind(job.status.as_str())
		.bind(job.attempts)
		.bind(job.max_attempts)
		.bind(format_dt(job.run_after))
		.bind(job.locked_by.as_deref())
		.bind(job.locked_at.map(format_dt))
		.bind(job.last_error.as_deref())
		.bind(format_dt(job.created_at))
		.bind(format_dt(job.updated_at))
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(())
	}

	async fn find_by_id(&self, id: &str) -> Result<Option<Job>> {
		let row = sqlx::query("SELECT * FROM jobs WHERE id = ?")
			.bind(id)
			.fetch_optional(&self.db)
			.await
			.map_err(db_error)?;
		map_row_opt(row, map_job)
	}

	async fn list(&self) -> Result<Vec<Job>> {
		let rows = sqlx::query("SELECT * FROM jobs ORDER BY created_at DESC")
			.fetch_all(&self.db)
			.await
			.map_err(db_error)?;
		map_rows(rows, map_job)
	}

	async fn claim_next(&self, worker_id: &str, now: DateTime<Utc>) -> Result<Option<Job>> {
		let now = format_dt(now);
		loop {
			let row = sqlx::query(
				"SELECT id FROM jobs WHERE status = 'pending' AND run_after <= ? \
				 ORDER BY run_after LIMIT 1",
			)
			.bind(&now)
			.fetch_optional(&self.db)
			.await
			.map_err(db_error)?;

			let Some(row) = row else {
				return Ok(None);
			};
			let id: String = row.try_get("id").map_err(db_error)?;

			let claimed = sqlx::query(
				"UPDATE jobs SET status = 'running', locked_by = ?, locked_at = ?, \
				 attempts = attempts + 1, updated_at = ? WHERE id = ? AND status = 'pending'",
			)
			.bind(worker_id)
			.bind(&now)
			.bind(&now)
			.bind(&id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;

			if claimed.rows_affected() == 0 {
				continue;
			}

			return self.find_by_id(&id).await;
		}
	}

	async fn complete(&self, id: &str, at: DateTime<Utc>) -> Result<()> {
		sqlx::query(
			"UPDATE jobs SET status = 'completed', locked_by = NULL, locked_at = NULL, \
			 updated_at = ? WHERE id = ?",
		)
		.bind(format_dt(at))
		.bind(id)
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(())
	}

	async fn reschedule(
		&self,
		id: &str,
		error: &str,
		run_after: DateTime<Utc>,
		at: DateTime<Utc>,
	) -> Result<()> {
		sqlx::query(
			"UPDATE jobs SET status = 'pending', run_after = ?, last_error = ?, \
			 locked_by = NULL, locked_at = NULL, updated_at = ? WHERE id = ?",
		)
		.bind(format_dt(run_after))
		.bind(error)
		.bind(format_dt(at))
		.bind(id)
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(())
	}

	async fn fail(&self, id: &str, error: &str, at: DateTime<Utc>) -> Result<()> {
		sqlx::query(
			"UPDATE jobs SET status = 'failed', last_error = ?, locked_by = NULL, \
			 locked_at = NULL, updated_at = ? WHERE id = ?",
		)
		.bind(error)
		.bind(format_dt(at))
		.bind(id)
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(())
	}

	async fn reap_stale(&self, older_than: DateTime<Utc>, at: DateTime<Utc>) -> Result<u64> {
		let reaped = sqlx::query(
			"UPDATE jobs SET status = 'pending', locked_by = NULL, locked_at = NULL, \
			 updated_at = ? WHERE status = 'running' AND locked_at < ?",
		)
		.bind(format_dt(at))
		.bind(format_dt(older_than))
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(reaped.rows_affected())
	}
}
