use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::models::MetadataCacheEntry;
use lunu_core::repo::MetadataCacheRepo;
use sqlx::Row;
use sqlx::any::AnyRow;

use super::map_row_opt;
use crate::convert::{format_dt, parse_dt};
use crate::{Db, db_error};

pub struct SqlxMetadataCacheRepo {
	db: Db,
}

impl SqlxMetadataCacheRepo {
	pub fn new(db: Db) -> Self {
		Self { db }
	}
}

fn map_entry(row: &AnyRow) -> Result<MetadataCacheEntry> {
	let fetched_at: String = row.try_get("fetched_at").map_err(db_error)?;

	Ok(MetadataCacheEntry {
		provider: row.try_get("provider").map_err(db_error)?,
		kind: row.try_get("kind").map_err(db_error)?,
		key: row.try_get("key").map_err(db_error)?,
		payload: row.try_get("payload").map_err(db_error)?,
		fetched_at: parse_dt(&fetched_at)?,
	})
}

#[async_trait]
impl MetadataCacheRepo for SqlxMetadataCacheRepo {
	async fn get(
		&self,
		provider: &str,
		kind: &str,
		key: &str,
	) -> Result<Option<MetadataCacheEntry>> {
		let row = sqlx::query(
			"SELECT * FROM metadata_cache WHERE provider = $1 AND kind = $2 AND key = $3",
		)
		.bind(provider)
		.bind(kind)
		.bind(key)
		.fetch_optional(&self.db)
		.await
		.map_err(db_error)?;
		map_row_opt(row, map_entry)
	}

	async fn put(&self, entry: &MetadataCacheEntry) -> Result<()> {
		sqlx::query(
			"INSERT INTO metadata_cache (provider, kind, key, payload, fetched_at) \
			 VALUES ($1, $2, $3, $4, $5) \
			 ON CONFLICT(provider, kind, key) DO UPDATE SET \
			 payload = excluded.payload, fetched_at = excluded.fetched_at",
		)
		.bind(&entry.provider)
		.bind(&entry.kind)
		.bind(&entry.key)
		.bind(&entry.payload)
		.bind(format_dt(entry.fetched_at))
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(())
	}
}
