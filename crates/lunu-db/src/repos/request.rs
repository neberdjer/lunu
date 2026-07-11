use async_trait::async_trait;
use chrono::{DateTime, Utc};
use lunu_core::Result;
use lunu_core::models::{Request, RequestStatus};
use lunu_core::repo::RequestRepo;
use sqlx::Row;
use sqlx::any::AnyRow;

use super::{map_row_opt, map_rows};
use crate::convert::{format_dt, parse_dt, parse_enum};
use crate::{Db, db_error, map_write_error};

pub struct SqlxRequestRepo {
	db: Db,
}

impl SqlxRequestRepo {
	pub fn new(db: Db) -> Self {
		Self { db }
	}
}

fn map_request(row: &AnyRow) -> Result<Request> {
	let status: String = row.try_get("status").map_err(db_error)?;
	let created_at: String = row.try_get("created_at").map_err(db_error)?;
	let updated_at: String = row.try_get("updated_at").map_err(db_error)?;

	Ok(Request {
		id: row.try_get("id").map_err(db_error)?,
		user_id: row.try_get("user_id").map_err(db_error)?,
		asin: row.try_get("asin").map_err(db_error)?,
		title: row.try_get("title").map_err(db_error)?,
		author: row.try_get("author").map_err(db_error)?,
		cover_url: row.try_get("cover_url").map_err(db_error)?,
		status: parse_enum::<RequestStatus>(&status)?,
		approved_by: row.try_get("approved_by").map_err(db_error)?,
		created_at: parse_dt(&created_at)?,
		updated_at: parse_dt(&updated_at)?,
	})
}

#[async_trait]
impl RequestRepo for SqlxRequestRepo {
	async fn create(&self, request: &Request) -> Result<()> {
		sqlx::query(
			"INSERT INTO requests \
			 (id, user_id, asin, title, author, cover_url, status, approved_by, created_at, updated_at) \
			 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
		)
		.bind(&request.id)
		.bind(&request.user_id)
		.bind(&request.asin)
		.bind(&request.title)
		.bind(request.author.as_deref())
		.bind(request.cover_url.as_deref())
		.bind(request.status.as_str())
		.bind(request.approved_by.as_deref())
		.bind(format_dt(request.created_at))
		.bind(format_dt(request.updated_at))
		.execute(&self.db)
		.await
		.map_err(map_write_error)?;
		Ok(())
	}

	async fn update(&self, request: &Request) -> Result<()> {
		sqlx::query(
			"UPDATE requests SET status = $1, approved_by = $2, updated_at = $3 WHERE id = $4",
		)
		.bind(request.status.as_str())
		.bind(request.approved_by.as_deref())
		.bind(format_dt(request.updated_at))
		.bind(&request.id)
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(())
	}

	async fn find_by_id(&self, id: &str) -> Result<Option<Request>> {
		let row = sqlx::query("SELECT * FROM requests WHERE id = $1")
			.bind(id)
			.fetch_optional(&self.db)
			.await
			.map_err(db_error)?;
		map_row_opt(row, map_request)
	}

	async fn find_by_user_and_asin(&self, user_id: &str, asin: &str) -> Result<Option<Request>> {
		let row = sqlx::query(
			"SELECT * FROM requests WHERE user_id = $1 AND asin = $2 ORDER BY created_at DESC LIMIT 1",
		)
		.bind(user_id)
		.bind(asin)
		.fetch_optional(&self.db)
		.await
		.map_err(db_error)?;
		map_row_opt(row, map_request)
	}

	async fn list(&self) -> Result<Vec<Request>> {
		let rows = sqlx::query("SELECT * FROM requests ORDER BY created_at DESC")
			.fetch_all(&self.db)
			.await
			.map_err(db_error)?;
		map_rows(rows, map_request)
	}

	async fn list_for_user(&self, user_id: &str) -> Result<Vec<Request>> {
		let rows =
			sqlx::query("SELECT * FROM requests WHERE user_id = $1 ORDER BY created_at DESC")
				.bind(user_id)
				.fetch_all(&self.db)
				.await
				.map_err(db_error)?;
		map_rows(rows, map_request)
	}

	async fn count_for_user_since(&self, user_id: &str, since: DateTime<Utc>) -> Result<i64> {
		let row = sqlx::query(
			"SELECT COUNT(*) AS count FROM requests WHERE user_id = $1 AND created_at >= $2",
		)
		.bind(user_id)
		.bind(format_dt(since))
		.fetch_one(&self.db)
		.await
		.map_err(db_error)?;
		row.try_get("count").map_err(db_error)
	}

	async fn delete(&self, id: &str) -> Result<()> {
		sqlx::query("DELETE FROM requests WHERE id = $1")
			.bind(id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}
}
