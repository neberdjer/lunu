use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::models::{Format, Watch};
use lunu_core::repo::WatchRepo;
use sqlx::Row;
use sqlx::any::AnyRow;

use super::{fetch_count, map_row_opt, map_rows};
use crate::convert::{format_dt, parse_dt, parse_enum};
use crate::{Db, db_error, map_write_error};

pub struct SqlxWatchRepo {
	db: Db,
}

impl SqlxWatchRepo {
	pub fn new(db: Db) -> Self {
		Self { db }
	}
}

const COLUMNS: &str = "id, user_id, work_id, format, asin, title, author, cover_url, \
	series_name, series_sequence, created_at";

fn map_watch(row: &AnyRow) -> Result<Watch> {
	let format: String = row.try_get("format").map_err(db_error)?;
	let created_at: String = row.try_get("created_at").map_err(db_error)?;

	Ok(Watch {
		id: row.try_get("id").map_err(db_error)?,
		user_id: row.try_get("user_id").map_err(db_error)?,
		work_id: row.try_get("work_id").map_err(db_error)?,
		format: parse_enum::<Format>(&format)?,
		asin: row.try_get("asin").map_err(db_error)?,
		title: row.try_get("title").map_err(db_error)?,
		author: row.try_get("author").map_err(db_error)?,
		cover_url: row.try_get("cover_url").map_err(db_error)?,
		series_name: row.try_get("series_name").map_err(db_error)?,
		series_sequence: row.try_get("series_sequence").map_err(db_error)?,
		created_at: parse_dt(&created_at)?,
	})
}

#[async_trait]
impl WatchRepo for SqlxWatchRepo {
	async fn create(&self, watch: &Watch) -> Result<()> {
		sqlx::query(&format!(
			"INSERT INTO watches ({COLUMNS}) \
			 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"
		))
		.bind(&watch.id)
		.bind(&watch.user_id)
		.bind(&watch.work_id)
		.bind(watch.format.as_str())
		.bind(watch.asin.as_deref())
		.bind(&watch.title)
		.bind(watch.author.as_deref())
		.bind(watch.cover_url.as_deref())
		.bind(watch.series_name.as_deref())
		.bind(watch.series_sequence.as_deref())
		.bind(format_dt(watch.created_at))
		.execute(&self.db)
		.await
		.map_err(map_write_error)?;
		Ok(())
	}

	async fn find_for_user(&self, user_id: &str, id: &str) -> Result<Option<Watch>> {
		let row = sqlx::query("SELECT * FROM watches WHERE id = $1 AND user_id = $2")
			.bind(id)
			.bind(user_id)
			.fetch_optional(&self.db)
			.await
			.map_err(db_error)?;
		map_row_opt(row, map_watch)
	}

	async fn list_page(&self, user_id: &str, limit: i64, offset: i64) -> Result<Vec<Watch>> {
		let rows = sqlx::query(
			"SELECT * FROM watches WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
		)
		.bind(user_id)
		.bind(limit)
		.bind(offset)
		.fetch_all(&self.db)
		.await
		.map_err(db_error)?;
		map_rows(rows, map_watch)
	}

	async fn count(&self, user_id: &str) -> Result<i64> {
		fetch_count(
			&self.db,
			sqlx::query("SELECT COUNT(*) AS count FROM watches WHERE user_id = $1").bind(user_id),
		)
		.await
	}

	async fn delete_owned(&self, user_id: &str, id: &str) -> Result<bool> {
		let result = sqlx::query("DELETE FROM watches WHERE id = $1 AND user_id = $2")
			.bind(id)
			.bind(user_id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(result.rows_affected() > 0)
	}
}
