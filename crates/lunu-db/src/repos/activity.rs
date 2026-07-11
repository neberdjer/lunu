use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::models::Activity;
use lunu_core::repo::ActivityRepo;
use sqlx::Row;
use sqlx::any::AnyRow;

use super::map_rows;
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
		event: row.try_get("event").map_err(db_error)?,
		detail: row.try_get("detail").map_err(db_error)?,
		at: parse_dt(&at)?,
	})
}

#[async_trait]
impl ActivityRepo for SqlxActivityRepo {
	async fn create(&self, activity: &Activity) -> Result<()> {
		sqlx::query(
			"INSERT INTO activity (id, request_id, event, detail, at) VALUES ($1, $2, $3, $4, $5)",
		)
		.bind(&activity.id)
		.bind(&activity.request_id)
		.bind(&activity.event)
		.bind(activity.detail.as_deref())
		.bind(format_dt(activity.at))
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(())
	}

	async fn recent(&self, limit: i64) -> Result<Vec<Activity>> {
		let rows = sqlx::query("SELECT * FROM activity ORDER BY at DESC LIMIT $1")
			.bind(limit)
			.fetch_all(&self.db)
			.await
			.map_err(db_error)?;
		map_rows(rows, map_activity)
	}

	async fn for_request(&self, request_id: &str) -> Result<Vec<Activity>> {
		let rows = sqlx::query("SELECT * FROM activity WHERE request_id = $1 ORDER BY at DESC")
			.bind(request_id)
			.fetch_all(&self.db)
			.await
			.map_err(db_error)?;
		map_rows(rows, map_activity)
	}
}
