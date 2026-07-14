use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::models::{Media, MediaSource};
use lunu_core::repo::MediaRepo;
use sqlx::Row;
use sqlx::any::AnyRow;

use super::{fetch_count, map_row_opt, map_rows};
use crate::convert::{bool_to_int, format_dt, int_to_bool, parse_dt, parse_enum};
use crate::{Db, db_error, map_write_error};

pub struct SqlxMediaRepo {
	db: Db,
}

impl SqlxMediaRepo {
	pub fn new(db: Db) -> Self {
		Self { db }
	}
}

const COLUMNS: &str = "id, asin, abs_item_id, title, author, cover_url, series_name, \
	series_sequence, library_path, source, overridden, request_id, created_at";
const VALUES: &str = "($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)";

fn list_filter(unmatched_only: bool) -> &'static str {
	if unmatched_only {
		"WHERE asin IS NULL"
	} else {
		""
	}
}

fn map_media(row: &AnyRow) -> Result<Media> {
	let created_at: String = row.try_get("created_at").map_err(db_error)?;
	let source: String = row.try_get("source").map_err(db_error)?;
	let overridden: i64 = row.try_get("overridden").map_err(db_error)?;

	Ok(Media {
		id: row.try_get("id").map_err(db_error)?,
		asin: row.try_get("asin").map_err(db_error)?,
		abs_item_id: row.try_get("abs_item_id").map_err(db_error)?,
		title: row.try_get("title").map_err(db_error)?,
		author: row.try_get("author").map_err(db_error)?,
		cover_url: row.try_get("cover_url").map_err(db_error)?,
		series_name: row.try_get("series_name").map_err(db_error)?,
		series_sequence: row.try_get("series_sequence").map_err(db_error)?,
		library_path: row.try_get("library_path").map_err(db_error)?,
		source: parse_enum::<MediaSource>(&source)?,
		overridden: int_to_bool(overridden),
		request_id: row.try_get("request_id").map_err(db_error)?,
		created_at: parse_dt(&created_at)?,
	})
}

async fn insert_media(db: &Db, sql: &str, media: &Media) -> Result<()> {
	sqlx::query(sql)
		.bind(&media.id)
		.bind(media.asin.as_deref())
		.bind(media.abs_item_id.as_deref())
		.bind(&media.title)
		.bind(media.author.as_deref())
		.bind(media.cover_url.as_deref())
		.bind(media.series_name.as_deref())
		.bind(media.series_sequence.as_deref())
		.bind(&media.library_path)
		.bind(media.source.as_str())
		.bind(bool_to_int(media.overridden))
		.bind(media.request_id.as_deref())
		.bind(format_dt(media.created_at))
		.execute(db)
		.await
		.map_err(map_write_error)?;
	Ok(())
}

#[async_trait]
impl MediaRepo for SqlxMediaRepo {
	async fn upsert_request(&self, media: &Media) -> Result<()> {
		let sql = format!(
			"INSERT INTO media ({COLUMNS}) VALUES {VALUES} \
			 ON CONFLICT (asin) DO UPDATE SET \
			 title = $4, author = $5, cover_url = $6, library_path = $9, request_id = $12"
		);
		insert_media(&self.db, &sql, media).await
	}

	async fn insert(&self, media: &Media) -> Result<()> {
		let sql = format!("INSERT INTO media ({COLUMNS}) VALUES {VALUES}");
		insert_media(&self.db, &sql, media).await
	}

	async fn update(&self, media: &Media) -> Result<()> {
		sqlx::query(
			"UPDATE media SET \
			 asin = $1, abs_item_id = $2, title = $3, author = $4, cover_url = $5, \
			 series_name = $6, series_sequence = $7, library_path = $8, overridden = $9 \
			 WHERE id = $10",
		)
		.bind(media.asin.as_deref())
		.bind(media.abs_item_id.as_deref())
		.bind(&media.title)
		.bind(media.author.as_deref())
		.bind(media.cover_url.as_deref())
		.bind(media.series_name.as_deref())
		.bind(media.series_sequence.as_deref())
		.bind(&media.library_path)
		.bind(bool_to_int(media.overridden))
		.bind(&media.id)
		.execute(&self.db)
		.await
		.map_err(map_write_error)?;
		Ok(())
	}

	async fn find_by_asin(&self, asin: &str) -> Result<Option<Media>> {
		let row = sqlx::query("SELECT * FROM media WHERE asin = $1 LIMIT 1")
			.bind(asin)
			.fetch_optional(&self.db)
			.await
			.map_err(db_error)?;
		map_row_opt(row, map_media)
	}

	async fn find_by_abs_item_id(&self, abs_item_id: &str) -> Result<Option<Media>> {
		let row = sqlx::query("SELECT * FROM media WHERE abs_item_id = $1")
			.bind(abs_item_id)
			.fetch_optional(&self.db)
			.await
			.map_err(db_error)?;
		map_row_opt(row, map_media)
	}

	async fn find_by_id(&self, id: &str) -> Result<Option<Media>> {
		let row = sqlx::query("SELECT * FROM media WHERE id = $1")
			.bind(id)
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

	async fn list_page(&self, unmatched_only: bool, limit: i64, offset: i64) -> Result<Vec<Media>> {
		let sql = format!(
			"SELECT * FROM media {} ORDER BY created_at DESC LIMIT $1 OFFSET $2",
			list_filter(unmatched_only)
		);
		let rows = sqlx::query(&sql)
			.bind(limit)
			.bind(offset)
			.fetch_all(&self.db)
			.await
			.map_err(db_error)?;
		map_rows(rows, map_media)
	}

	async fn list_count(&self, unmatched_only: bool) -> Result<i64> {
		let sql = format!(
			"SELECT COUNT(*) AS count FROM media {}",
			list_filter(unmatched_only)
		);
		fetch_count(&self.db, sqlx::query(&sql)).await
	}

	async fn count(&self) -> Result<i64> {
		fetch_count(&self.db, sqlx::query("SELECT COUNT(*) AS count FROM media")).await
	}
}
