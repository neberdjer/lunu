use std::str::FromStr;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use lunu_core::Result;
use lunu_core::models::{Request, RequestStatus};
use lunu_core::repo::RequestRepo;
use sqlx::Row;

use super::{fetch_count, map_row_opt, map_rows};
use crate::convert::format_dt;
use crate::{Db, db_error, map_write_error};

mod row;

use row::{map_request, request_filter};

pub struct SqlxRequestRepo {
	db: Db,
}

impl SqlxRequestRepo {
	pub fn new(db: Db) -> Self {
		Self { db }
	}
}

#[async_trait]
impl RequestRepo for SqlxRequestRepo {
	async fn create(&self, request: &Request) -> Result<()> {
		sqlx::query(
			"INSERT INTO requests \
			 (id, user_id, work_id, format, asin, title, author, cover_url, status, approved_by, notes, quality_profile_id, created_at, updated_at) \
			 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
		)
		.bind(&request.id)
		.bind(&request.user_id)
		.bind(&request.work_id)
		.bind(request.format.as_str())
		.bind(request.asin.as_deref())
		.bind(&request.title)
		.bind(request.author.as_deref())
		.bind(request.cover_url.as_deref())
		.bind(request.status.as_str())
		.bind(request.approved_by.as_deref())
		.bind(request.notes.as_deref())
		.bind(request.quality_profile_id.as_deref())
		.bind(format_dt(request.created_at))
		.bind(format_dt(request.updated_at))
		.execute(&self.db)
		.await
		.map_err(map_write_error)?;
		Ok(())
	}

	async fn create_within_quota(
		&self,
		request: &Request,
		quota: i64,
		since: DateTime<Utc>,
	) -> Result<bool> {
		let result = sqlx::query(
			"INSERT INTO requests \
			 (id, user_id, work_id, format, asin, title, author, cover_url, status, approved_by, notes, quality_profile_id, created_at, updated_at) \
			 SELECT $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14 \
			 WHERE (SELECT COUNT(*) FROM requests WHERE user_id = $2 AND created_at >= $15) < $16",
		)
		.bind(&request.id)
		.bind(&request.user_id)
		.bind(&request.work_id)
		.bind(request.format.as_str())
		.bind(request.asin.as_deref())
		.bind(&request.title)
		.bind(request.author.as_deref())
		.bind(request.cover_url.as_deref())
		.bind(request.status.as_str())
		.bind(request.approved_by.as_deref())
		.bind(request.notes.as_deref())
		.bind(request.quality_profile_id.as_deref())
		.bind(format_dt(request.created_at))
		.bind(format_dt(request.updated_at))
		.bind(format_dt(since))
		.bind(quota)
		.execute(&self.db)
		.await
		.map_err(map_write_error)?;
		Ok(result.rows_affected() > 0)
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

	async fn transition_if_pending(
		&self,
		id: &str,
		status: &str,
		approved_by: &str,
		at: chrono::DateTime<chrono::Utc>,
	) -> Result<bool> {
		let result = sqlx::query(
			"UPDATE requests SET status = $1, approved_by = $2, updated_at = $3 \
			 WHERE id = $4 AND status = 'pending'",
		)
		.bind(status)
		.bind(approved_by)
		.bind(format_dt(at))
		.bind(id)
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(result.rows_affected() > 0)
	}

	async fn find_by_id(&self, id: &str) -> Result<Option<Request>> {
		let row = sqlx::query("SELECT * FROM requests WHERE id = $1")
			.bind(id)
			.fetch_optional(&self.db)
			.await
			.map_err(db_error)?;
		map_row_opt(row, map_request)
	}

	async fn find_by_user_and_work(&self, user_id: &str, work_id: &str) -> Result<Option<Request>> {
		let row = sqlx::query(
			"SELECT * FROM requests WHERE user_id = $1 AND work_id = $2 \
			 ORDER BY created_at DESC LIMIT 1",
		)
		.bind(user_id)
		.bind(work_id)
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

	async fn status_by_works(
		&self,
		user_id: &str,
		work_ids: &[String],
	) -> Result<Vec<(String, RequestStatus)>> {
		if work_ids.is_empty() {
			return Ok(Vec::new());
		}
		let placeholders: Vec<String> = (2..=work_ids.len() + 1).map(|n| format!("${n}")).collect();
		let sql = format!(
			"SELECT work_id, status FROM requests WHERE user_id = $1 AND work_id IN ({}) \
			 ORDER BY created_at DESC",
			placeholders.join(", ")
		);
		let mut query = sqlx::query(&sql).bind(user_id);
		for work_id in work_ids {
			query = query.bind(work_id);
		}
		let rows = query.fetch_all(&self.db).await.map_err(db_error)?;

		let mut statuses = Vec::with_capacity(rows.len());
		for row in rows {
			let work_id: String = row.try_get("work_id").map_err(db_error)?;
			let status: String = row.try_get("status").map_err(db_error)?;
			statuses.push((work_id, RequestStatus::from_str(&status)?));
		}
		Ok(statuses)
	}

	async fn status_by_asins(
		&self,
		user_id: &str,
		asins: &[String],
	) -> Result<Vec<(String, RequestStatus)>> {
		if asins.is_empty() {
			return Ok(Vec::new());
		}
		let placeholders: Vec<String> = (2..=asins.len() + 1).map(|n| format!("${n}")).collect();
		let sql = format!(
			"SELECT asin, status FROM requests WHERE user_id = $1 AND asin IN ({}) \
			 ORDER BY created_at DESC",
			placeholders.join(", ")
		);
		let mut query = sqlx::query(&sql).bind(user_id);
		for asin in asins {
			query = query.bind(asin);
		}
		let rows = query.fetch_all(&self.db).await.map_err(db_error)?;

		let mut statuses = Vec::with_capacity(rows.len());
		for row in rows {
			let asin: String = row.try_get("asin").map_err(db_error)?;
			let status: String = row.try_get("status").map_err(db_error)?;
			statuses.push((asin, RequestStatus::from_str(&status)?));
		}
		Ok(statuses)
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

	async fn list_page(
		&self,
		user_id: Option<&str>,
		status: Option<&str>,
		limit: i64,
		offset: i64,
	) -> Result<Vec<Request>> {
		let (where_clause, next) = request_filter(user_id, status);
		let sql = format!(
			"SELECT * FROM requests{where_clause} ORDER BY created_at DESC LIMIT ${next} OFFSET ${}",
			next + 1
		);
		let mut query = sqlx::query(&sql);
		if let Some(user_id) = user_id {
			query = query.bind(user_id);
		}
		if let Some(status) = status {
			query = query.bind(status);
		}
		let rows = query
			.bind(limit)
			.bind(offset)
			.fetch_all(&self.db)
			.await
			.map_err(db_error)?;
		map_rows(rows, map_request)
	}

	async fn count(&self, user_id: Option<&str>, status: Option<&str>) -> Result<i64> {
		let (where_clause, _) = request_filter(user_id, status);
		let sql = format!("SELECT COUNT(*) AS count FROM requests{where_clause}");
		let mut query = sqlx::query(&sql);
		if let Some(user_id) = user_id {
			query = query.bind(user_id);
		}
		if let Some(status) = status {
			query = query.bind(status);
		}
		fetch_count(&self.db, query).await
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
