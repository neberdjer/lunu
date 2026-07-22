use chrono::{DateTime, Utc};
use lunu_core::Result;
use lunu_core::models::{Job, JobType};
use sqlx::Row;

use super::map_job;
use crate::repos::placeholders;
use crate::{Db, convert::format_dt, db_error};

pub(super) async fn claim_next(
	db: &Db,
	worker_id: &str,
	now: DateTime<Utc>,
	lane: &[JobType],
) -> Result<Option<Job>> {
	if lane.is_empty() {
		return Ok(None);
	}
	let now = format_dt(now);
	let sql = format!(
		"SELECT id FROM jobs WHERE status = 'pending' AND run_after <= $1 \
		 AND job_type IN ({}) ORDER BY run_after LIMIT 1",
		placeholders(2, lane.len())
	);
	loop {
		let mut query = sqlx::query(&sql).bind(&now);
		for job_type in lane {
			query = query.bind(job_type.as_str());
		}
		let row = query.fetch_optional(db).await.map_err(db_error)?;

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
		.execute(db)
		.await
		.map_err(db_error)?;

		if claimed.rows_affected() == 0 {
			continue;
		}

		let row = sqlx::query("SELECT * FROM jobs WHERE id = $1")
			.bind(&id)
			.fetch_optional(db)
			.await
			.map_err(db_error)?;
		return crate::repos::map_row_opt(row, map_job);
	}
}
