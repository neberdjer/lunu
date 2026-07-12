use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::models::BlocklistEntry;
use lunu_core::repo::BlocklistRepo;
use sqlx::Row;
use sqlx::any::AnyRow;

use super::map_rows;
use crate::convert::{format_dt, parse_dt};
use crate::{Db, db_error, map_write_error};

fn map_entry(row: &AnyRow) -> Result<BlocklistEntry> {
	let created_at: String = row.try_get("created_at").map_err(db_error)?;
	Ok(BlocklistEntry {
		id: row.try_get("id").map_err(db_error)?,
		request_id: row.try_get("request_id").map_err(db_error)?,
		download_url: row.try_get("download_url").map_err(db_error)?,
		created_at: parse_dt(&created_at)?,
	})
}

pub struct SqlxBlocklistRepo {
	db: Db,
}

impl SqlxBlocklistRepo {
	pub fn new(db: Db) -> Self {
		Self { db }
	}
}

#[async_trait]
impl BlocklistRepo for SqlxBlocklistRepo {
	async fn add(&self, entry: &BlocklistEntry) -> Result<()> {
		sqlx::query(
			"INSERT INTO blocklist (id, request_id, download_url, created_at) VALUES ($1, $2, $3, $4)",
		)
		.bind(&entry.id)
		.bind(&entry.request_id)
		.bind(&entry.download_url)
		.bind(format_dt(entry.created_at))
		.execute(&self.db)
		.await
		.map_err(map_write_error)?;
		Ok(())
	}

	async fn urls_for_request(&self, request_id: &str) -> Result<Vec<String>> {
		let rows = sqlx::query("SELECT download_url FROM blocklist WHERE request_id = $1")
			.bind(request_id)
			.fetch_all(&self.db)
			.await
			.map_err(db_error)?;
		rows.iter()
			.map(|row| row.try_get("download_url").map_err(db_error))
			.collect()
	}

	async fn list_for_request(&self, request_id: &str) -> Result<Vec<BlocklistEntry>> {
		let rows =
			sqlx::query("SELECT * FROM blocklist WHERE request_id = $1 ORDER BY created_at DESC")
				.bind(request_id)
				.fetch_all(&self.db)
				.await
				.map_err(db_error)?;
		map_rows(rows, map_entry)
	}

	async fn remove(&self, request_id: &str, download_url: &str) -> Result<bool> {
		let result =
			sqlx::query("DELETE FROM blocklist WHERE request_id = $1 AND download_url = $2")
				.bind(request_id)
				.bind(download_url)
				.execute(&self.db)
				.await
				.map_err(db_error)?;
		Ok(result.rows_affected() > 0)
	}
}
