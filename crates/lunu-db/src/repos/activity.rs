use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::models::Activity;
use lunu_core::repo::ActivityRepo;
use sqlx::Row;
use sqlx::any::AnyRow;

use super::{fetch_count, map_rows};
use crate::convert::{format_dt, parse_dt};
use crate::{Db, db_error};

pub struct SqlxActivityRepo {
	db: Db,
}

impl SqlxActivityRepo {
	pub fn new(db: Db) -> Self {
		Self { db }
	}
}

fn map_activity(row: &AnyRow) -> Result<Activity> {
	let at: String = row.try_get("at").map_err(db_error)?;

	Ok(Activity {
		id: row.try_get("id").map_err(db_error)?,
		request_id: row.try_get("request_id").map_err(db_error)?,
		media_id: row.try_get("media_id").map_err(db_error)?,
		event: row.try_get("event").map_err(db_error)?,
		detail: row.try_get("detail").map_err(db_error)?,
		actor: row.try_get("actor").map_err(db_error)?,
		at: parse_dt(&at)?,
	})
}

#[async_trait]
impl ActivityRepo for SqlxActivityRepo {
	async fn create(&self, activity: &Activity) -> Result<()> {
		sqlx::query(
			"INSERT INTO activity (id, request_id, media_id, event, detail, actor, at) \
			 VALUES ($1, $2, $3, $4, $5, $6, $7)",
		)
		.bind(&activity.id)
		.bind(activity.request_id.as_deref())
		.bind(activity.media_id.as_deref())
		.bind(&activity.event)
		.bind(activity.detail.as_deref())
		.bind(activity.actor.as_deref())
		.bind(format_dt(activity.at))
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(())
	}

	async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<Activity>> {
		let rows = sqlx::query("SELECT * FROM activity ORDER BY at DESC LIMIT $1 OFFSET $2")
			.bind(limit)
			.bind(offset)
			.fetch_all(&self.db)
			.await
			.map_err(db_error)?;
		map_rows(rows, map_activity)
	}

	async fn count(&self) -> Result<i64> {
		fetch_count(
			&self.db,
			sqlx::query("SELECT COUNT(*) AS count FROM activity"),
		)
		.await
	}

	async fn for_request(&self, request_id: &str) -> Result<Vec<Activity>> {
		let rows = sqlx::query("SELECT * FROM activity WHERE request_id = $1 ORDER BY at DESC")
			.bind(request_id)
			.fetch_all(&self.db)
			.await
			.map_err(db_error)?;
		map_rows(rows, map_activity)
	}

	async fn delete_for_request(&self, request_id: &str) -> Result<()> {
		sqlx::query("DELETE FROM activity WHERE request_id = $1")
			.bind(request_id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}

	async fn delete_before(&self, cutoff: chrono::DateTime<chrono::Utc>) -> Result<u64> {
		let result = sqlx::query("DELETE FROM activity WHERE at < $1")
			.bind(format_dt(cutoff))
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(result.rows_affected())
	}
}
