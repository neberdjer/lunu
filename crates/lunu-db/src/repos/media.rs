use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::models::Media;
use lunu_core::repo::MediaRepo;
use sqlx::Row;
use sqlx::any::AnyRow;

use super::{fetch_count, map_row_opt};
use crate::convert::{format_dt, parse_dt};
use crate::{Db, db_error, map_write_error};

pub struct SqlxMediaRepo {
	db: Db,
}

impl SqlxMediaRepo {
	pub fn new(db: Db) -> Self {
		Self { db }
	}
}

fn map_media(row: &AnyRow) -> Result<Media> {
	let created_at: String = row.try_get("created_at").map_err(db_error)?;

	Ok(Media {
		asin: row.try_get("asin").map_err(db_error)?,
		title: row.try_get("title").map_err(db_error)?,
		author: row.try_get("author").map_err(db_error)?,
		cover_url: row.try_get("cover_url").map_err(db_error)?,
		library_path: row.try_get("library_path").map_err(db_error)?,
		request_id: row.try_get("request_id").map_err(db_error)?,
		created_at: parse_dt(&created_at)?,
	})
}

#[async_trait]
impl MediaRepo for SqlxMediaRepo {
	async fn upsert(&self, media: &Media) -> Result<()> {
		sqlx::query(
			"INSERT INTO media (asin, title, author, cover_url, library_path, request_id, created_at) \
			 VALUES ($1, $2, $3, $4, $5, $6, $7) \
			 ON CONFLICT (asin) DO UPDATE SET \
			 title = $2, author = $3, cover_url = $4, library_path = $5, request_id = $6",
		)
		.bind(&media.asin)
		.bind(&media.title)
		.bind(media.author.as_deref())
		.bind(media.cover_url.as_deref())
		.bind(&media.library_path)
		.bind(media.request_id.as_deref())
		.bind(format_dt(media.created_at))
		.execute(&self.db)
		.await
		.map_err(map_write_error)?;
		Ok(())
	}

	async fn find_by_asin(&self, asin: &str) -> Result<Option<Media>> {
		let row = sqlx::query("SELECT * FROM media WHERE asin = $1")
			.bind(asin)
			.fetch_optional(&self.db)
			.await
			.map_err(db_error)?;
		map_row_opt(row, map_media)
	}

	async fn available_among(&self, asins: &[String]) -> Result<Vec<String>> {
		if asins.is_empty() {
			return Ok(Vec::new());
		}
		let placeholders: Vec<String> = (1..=asins.len()).map(|n| format!("${n}")).collect();
		let sql = format!(
			"SELECT asin FROM media WHERE asin IN ({})",
			placeholders.join(", ")
		);
		let mut query = sqlx::query(&sql);
		for asin in asins {
			query = query.bind(asin);
		}
		let rows = query.fetch_all(&self.db).await.map_err(db_error)?;
		rows.iter()
			.map(|row| row.try_get::<String, _>("asin").map_err(db_error))
			.collect()
	}

	async fn count(&self) -> Result<i64> {
		fetch_count(&self.db, sqlx::query("SELECT COUNT(*) AS count FROM media")).await
	}
}
