use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::models::BlocklistEntry;
use lunu_core::repo::BlocklistRepo;
use sqlx::Row;

use crate::convert::format_dt;
use crate::{Db, db_error, map_write_error};

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
}
