use async_trait::async_trait;
use chrono::{DateTime, Utc};
use lunu_core::Result;
use lunu_core::models::{Download, DownloadState};
use lunu_core::repo::DownloadRepo;
use sqlx::Row;
use sqlx::any::AnyRow;

use super::{fetch_count, map_row_opt, map_rows};
use crate::convert::{format_dt, parse_dt, parse_enum};
use crate::{Db, db_error};

pub struct SqlxDownloadRepo {
	db: Db,
}

impl SqlxDownloadRepo {
	pub fn new(db: Db) -> Self {
		Self { db }
	}
}

fn map_download(row: &AnyRow) -> Result<Download> {
	let state: String = row.try_get("state").map_err(db_error)?;
	let created_at: String = row.try_get("created_at").map_err(db_error)?;
	let updated_at: String = row.try_get("updated_at").map_err(db_error)?;

	Ok(Download {
		id: row.try_get("id").map_err(db_error)?,
		request_id: row.try_get("request_id").map_err(db_error)?,
		client: row.try_get("client").map_err(db_error)?,
		category: row.try_get("category").map_err(db_error)?,
		release_title: row.try_get("release_title").map_err(db_error)?,
		indexer: row.try_get("indexer").map_err(db_error)?,
		download_url: row.try_get("download_url").map_err(db_error)?,
		client_ref: row.try_get("client_ref").map_err(db_error)?,
		state: parse_enum::<DownloadState>(&state)?,
		progress: row.try_get("progress").map_err(db_error)?,
		created_at: parse_dt(&created_at)?,
		updated_at: parse_dt(&updated_at)?,
	})
}

#[async_trait]
impl DownloadRepo for SqlxDownloadRepo {
	async fn create(&self, download: &Download) -> Result<()> {
		sqlx::query(
			"INSERT INTO downloads \
			 (id, request_id, client, category, release_title, indexer, download_url, client_ref, state, progress, created_at, updated_at) \
			 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
		)
		.bind(&download.id)
		.bind(&download.request_id)
		.bind(&download.client)
		.bind(&download.category)
		.bind(&download.release_title)
		.bind(&download.indexer)
		.bind(&download.download_url)
		.bind(download.client_ref.as_deref())
		.bind(download.state.as_str())
		.bind(download.progress)
		.bind(format_dt(download.created_at))
		.bind(format_dt(download.updated_at))
		.execute(&self.db)
		.await
		.map_err(crate::map_write_error)?;
		Ok(())
	}

	async fn find_by_id(&self, id: &str) -> Result<Option<Download>> {
		let row = sqlx::query("SELECT * FROM downloads WHERE id = $1")
			.bind(id)
			.fetch_optional(&self.db)
			.await
			.map_err(db_error)?;
		map_row_opt(row, map_download)
	}

	async fn find_by_request(&self, request_id: &str) -> Result<Option<Download>> {
		let row = sqlx::query(
			"SELECT * FROM downloads WHERE request_id = $1 ORDER BY created_at DESC LIMIT 1",
		)
		.bind(request_id)
		.fetch_optional(&self.db)
		.await
		.map_err(db_error)?;
		map_row_opt(row, map_download)
	}

	async fn list(&self) -> Result<Vec<Download>> {
		let rows = sqlx::query("SELECT * FROM downloads ORDER BY created_at DESC")
			.fetch_all(&self.db)
			.await
			.map_err(db_error)?;
		map_rows(rows, map_download)
	}

	async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<Download>> {
		let rows =
			sqlx::query("SELECT * FROM downloads ORDER BY created_at DESC LIMIT $1 OFFSET $2")
				.bind(limit)
				.bind(offset)
				.fetch_all(&self.db)
				.await
				.map_err(db_error)?;
		map_rows(rows, map_download)
	}

	async fn count(&self) -> Result<i64> {
		fetch_count(
			&self.db,
			sqlx::query("SELECT COUNT(*) AS count FROM downloads"),
		)
		.await
	}

	async fn update_status(
		&self,
		id: &str,
		state: DownloadState,
		progress: i64,
		at: DateTime<Utc>,
	) -> Result<()> {
		sqlx::query(
			"UPDATE downloads SET state = $1, progress = $2, updated_at = $3 WHERE id = $4",
		)
		.bind(state.as_str())
		.bind(progress)
		.bind(format_dt(at))
		.bind(id)
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(())
	}

	async fn delete_for_request(&self, request_id: &str) -> Result<()> {
		sqlx::query("DELETE FROM downloads WHERE request_id = $1")
			.bind(request_id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}
}
