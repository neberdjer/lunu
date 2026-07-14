use async_trait::async_trait;
use chrono::{DateTime, Utc};
use lunu_core::Result;
use lunu_core::models::Schedule;
use lunu_core::repo::ScheduleRepo;
use sqlx::Row;
use sqlx::any::AnyRow;

use super::map_rows;
use crate::convert::{bool_to_int, format_dt, int_to_bool, parse_dt, parse_dt_opt};
use crate::{Db, db_error, map_write_error};

pub struct SqlxScheduleRepo {
	db: Db,
}

impl SqlxScheduleRepo {
	pub fn new(db: Db) -> Self {
		Self { db }
	}
}

fn map_schedule(row: &AnyRow) -> Result<Schedule> {
	let enabled: i64 = row.try_get("enabled").map_err(db_error)?;
	let next_run_at: String = row.try_get("next_run_at").map_err(db_error)?;
	let last_run_at: Option<String> = row.try_get("last_run_at").map_err(db_error)?;
	let updated_at: String = row.try_get("updated_at").map_err(db_error)?;

	Ok(Schedule {
		kind: row.try_get("kind").map_err(db_error)?,
		interval_secs: row.try_get("interval_secs").map_err(db_error)?,
		enabled: int_to_bool(enabled),
		next_run_at: parse_dt(&next_run_at)?,
		last_run_at: parse_dt_opt(last_run_at)?,
		updated_at: parse_dt(&updated_at)?,
	})
}

#[async_trait]
impl ScheduleRepo for SqlxScheduleRepo {
	async fn insert_if_absent(&self, schedule: &Schedule) -> Result<()> {
		sqlx::query(
			"INSERT INTO schedules \
			 (kind, interval_secs, enabled, next_run_at, last_run_at, updated_at) \
			 VALUES ($1, $2, $3, $4, $5, $6) \
			 ON CONFLICT (kind) DO NOTHING",
		)
		.bind(&schedule.kind)
		.bind(schedule.interval_secs)
		.bind(bool_to_int(schedule.enabled))
		.bind(format_dt(schedule.next_run_at))
		.bind(schedule.last_run_at.map(format_dt))
		.bind(format_dt(schedule.updated_at))
		.execute(&self.db)
		.await
		.map_err(map_write_error)?;
		Ok(())
	}

	async fn list(&self) -> Result<Vec<Schedule>> {
		let rows = sqlx::query("SELECT * FROM schedules ORDER BY kind")
			.fetch_all(&self.db)
			.await
			.map_err(db_error)?;
		map_rows(rows, map_schedule)
	}

	async fn due(&self, now: DateTime<Utc>) -> Result<Vec<Schedule>> {
		let rows = sqlx::query(
			"SELECT * FROM schedules WHERE enabled = $1 AND next_run_at <= $2 ORDER BY kind",
		)
		.bind(bool_to_int(true))
		.bind(format_dt(now))
		.fetch_all(&self.db)
		.await
		.map_err(db_error)?;
		map_rows(rows, map_schedule)
	}

	async fn advance(
		&self,
		kind: &str,
		last_run: DateTime<Utc>,
		next_run: DateTime<Utc>,
	) -> Result<()> {
		sqlx::query(
			"UPDATE schedules SET last_run_at = $1, next_run_at = $2, updated_at = $3 \
			 WHERE kind = $4",
		)
		.bind(format_dt(last_run))
		.bind(format_dt(next_run))
		.bind(format_dt(Utc::now()))
		.bind(kind)
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(())
	}

	async fn configure(
		&self,
		kind: &str,
		enabled: bool,
		interval_secs: i64,
		next_run: DateTime<Utc>,
	) -> Result<bool> {
		let result = sqlx::query(
			"UPDATE schedules SET enabled = $1, interval_secs = $2, next_run_at = $3, \
			 updated_at = $4 WHERE kind = $5",
		)
		.bind(bool_to_int(enabled))
		.bind(interval_secs)
		.bind(format_dt(next_run))
		.bind(format_dt(Utc::now()))
		.bind(kind)
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(result.rows_affected() > 0)
	}
}
