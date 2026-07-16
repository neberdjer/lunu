use async_trait::async_trait;
use chrono::{DateTime, Utc};
use lunu_core::Result;
use lunu_core::models::{Job, JobStatus, JobType};
use lunu_core::repo::JobRepo;
use sqlx::Row;
use sqlx::any::AnyRow;

use super::{count_by_status, list_by_status, map_row_opt, map_rows};
use crate::convert::{format_dt, parse_dt, parse_dt_opt, parse_enum};
use crate::{Db, db_error};

pub struct SqlxJobRepo {
	db: Db,
}

impl SqlxJobRepo {
	pub fn new(db: Db) -> Self {
		Self { db }
	}

	async fn insert_job(&self, sql: &str, job: &Job) -> Result<u64> {
		let result = sqlx::query(sql)
			.bind(&job.id)
			.bind(job.job_type.as_str())
			.bind(job.request_id.as_deref())
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
		Ok(result.rows_affected())
	}
}

const INSERT_JOB: &str = "INSERT INTO jobs \
	 (id, job_type, request_id, payload, status, attempts, max_attempts, run_after, locked_by, locked_at, last_error, created_at, updated_at) \
	 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)";

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
		request_id: row.try_get("request_id").map_err(db_error)?,
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
		self.insert_job(INSERT_JOB, job).await?;
		Ok(())
	}

	async fn create_recurring(&self, job: &Job) -> Result<bool> {
		let inserted = self
			.insert_job(&format!("{INSERT_JOB} ON CONFLICT DO NOTHING"), job)
			.await?;
		Ok(inserted > 0)
	}

	async fn find_by_id(&self, id: &str) -> Result<Option<Job>> {
		let row = sqlx::query("SELECT * FROM jobs WHERE id = $1")
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

	async fn list_page(&self, status: Option<&str>, limit: i64, offset: i64) -> Result<Vec<Job>> {
		list_by_status(&self.db, "jobs", status, limit, offset, map_job).await
	}

	async fn count(&self, status: Option<&str>) -> Result<i64> {
		count_by_status(&self.db, "jobs", status).await
	}

	async fn requeue(&self, id: &str, at: DateTime<Utc>) -> Result<bool> {
		let result = sqlx::query(
			"UPDATE jobs SET status = 'pending', attempts = 0, run_after = $1, \
			 last_error = NULL, locked_by = NULL, locked_at = NULL, updated_at = $1 \
			 WHERE id = $2 AND status = 'failed'",
		)
		.bind(format_dt(at))
		.bind(id)
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(result.rows_affected() > 0)
	}

	async fn delete(&self, id: &str) -> Result<bool> {
		let result = sqlx::query("DELETE FROM jobs WHERE id = $1")
			.bind(id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(result.rows_affected() > 0)
	}

	async fn has_active(&self, job_type: &str, request_id: &str) -> Result<bool> {
		let row = sqlx::query(
			"SELECT id FROM jobs WHERE job_type = $1 AND request_id = $2 \
			 AND status IN ('pending', 'running') LIMIT 1",
		)
		.bind(job_type)
		.bind(request_id)
		.fetch_optional(&self.db)
		.await
		.map_err(db_error)?;
		Ok(row.is_some())
	}

	async fn claim_next(&self, worker_id: &str, now: DateTime<Utc>) -> Result<Option<Job>> {
		let now = format_dt(now);
		loop {
			let row = sqlx::query(
				"SELECT id FROM jobs WHERE status = 'pending' AND run_after <= $1 \
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
				"UPDATE jobs SET status = 'running', locked_by = $1, locked_at = $2, \
				 attempts = attempts + 1, updated_at = $3 \
				 WHERE id = $4 AND status = 'pending' AND run_after <= $5",
			)
			.bind(worker_id)
			.bind(&now)
			.bind(&now)
			.bind(&id)
			.bind(&now)
			.execute(&self.db)
			.await
			.map_err(db_error)?;

			if claimed.rows_affected() == 0 {
				continue;
			}

			return self.find_by_id(&id).await;
		}
	}

	async fn renew_lease(&self, id: &str, locked_by: &str, now: DateTime<Utc>) -> Result<bool> {
		let result = sqlx::query(
			"UPDATE jobs SET locked_at = $1, updated_at = $1 \
			 WHERE id = $2 AND locked_by = $3 AND status = 'running'",
		)
		.bind(format_dt(now))
		.bind(id)
		.bind(locked_by)
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(result.rows_affected() > 0)
	}

	async fn complete(&self, id: &str, locked_by: &str, at: DateTime<Utc>) -> Result<()> {
		sqlx::query(
			"UPDATE jobs SET status = 'completed', locked_by = NULL, locked_at = NULL, \
			 updated_at = $1 WHERE id = $2 AND locked_by = $3 AND status = 'running'",
		)
		.bind(format_dt(at))
		.bind(id)
		.bind(locked_by)
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(())
	}

	async fn reschedule(
		&self,
		id: &str,
		locked_by: &str,
		error: &str,
		run_after: DateTime<Utc>,
		at: DateTime<Utc>,
		max_attempts: i64,
	) -> Result<()> {
		sqlx::query(
			"UPDATE jobs SET status = 'pending', run_after = $1, last_error = $2, \
			 max_attempts = $3, locked_by = NULL, locked_at = NULL, updated_at = $4 \
			 WHERE id = $5 AND locked_by = $6 AND status = 'running'",
		)
		.bind(format_dt(run_after))
		.bind(error)
		.bind(max_attempts)
		.bind(format_dt(at))
		.bind(id)
		.bind(locked_by)
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(())
	}

	async fn fail(&self, id: &str, locked_by: &str, error: &str, at: DateTime<Utc>) -> Result<()> {
		sqlx::query(
			"UPDATE jobs SET status = 'failed', last_error = $1, locked_by = NULL, \
			 locked_at = NULL, updated_at = $2 WHERE id = $3 AND locked_by = $4 AND status = 'running'",
		)
		.bind(error)
		.bind(format_dt(at))
		.bind(id)
		.bind(locked_by)
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(())
	}

	async fn delete_finished_before(&self, cutoff: DateTime<Utc>) -> Result<u64> {
		let deleted = sqlx::query(
			"DELETE FROM jobs WHERE status IN ('completed', 'failed') AND updated_at < $1",
		)
		.bind(format_dt(cutoff))
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(deleted.rows_affected())
	}

	async fn reap_stale(&self, older_than: DateTime<Utc>, at: DateTime<Utc>) -> Result<u64> {
		let reaped = sqlx::query(
			"UPDATE jobs SET status = 'pending', locked_by = NULL, locked_at = NULL, \
			 updated_at = $1 WHERE status = 'running' AND locked_at < $2",
		)
		.bind(format_dt(at))
		.bind(format_dt(older_than))
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(reaped.rows_affected())
	}
}
