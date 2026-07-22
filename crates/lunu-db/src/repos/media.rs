use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::models::{Format, MatchedBy, Media, MediaFilter, MediaSource, MergeState};
use lunu_core::repo::MediaRepo;
use sqlx::Row;
use sqlx::any::AnyRow;

use super::{fetch_count, map_row_opt, map_rows, placeholders};
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

const COLUMNS: &str = "id, work_id, format, asin, abs_item_id, title, author, cover_url, \
	series_name, series_sequence, library_path, merged_path, merge_state, merge_detail, \
	merge_backup_path, source, overridden, matched_by, request_id, created_at";
const COLUMN_COUNT: usize = 20;

fn list_filter(filter: MediaFilter) -> String {
	match filter {
		MediaFilter::All => String::new(),
		MediaFilter::Unmatched => "WHERE matched_by IS NULL AND source = 'abs'".to_string(),
		MediaFilter::Mergeable => {
			let candidates: Vec<String> = MergeState::ALL
				.iter()
				.filter(|state| state.is_merge_candidate())
				.map(|state| format!("'{}'", state.as_str()))
				.collect();
			format!(
				"WHERE library_path <> '' AND merge_state IN ({})",
				candidates.join(", ")
			)
		}
	}
}

fn map_media(row: &AnyRow) -> Result<Media> {
	let created_at: String = row.try_get("created_at").map_err(db_error)?;
	let source: String = row.try_get("source").map_err(db_error)?;
	let format: String = row.try_get("format").map_err(db_error)?;
	let matched_by: Option<String> = row.try_get("matched_by").map_err(db_error)?;
	let overridden: i64 = row.try_get("overridden").map_err(db_error)?;
	let merge_state: String = row.try_get("merge_state").map_err(db_error)?;

	Ok(Media {
		id: row.try_get("id").map_err(db_error)?,
		work_id: row.try_get("work_id").map_err(db_error)?,
		format: parse_enum::<Format>(&format)?,
		asin: row.try_get("asin").map_err(db_error)?,
		abs_item_id: row.try_get("abs_item_id").map_err(db_error)?,
		title: row.try_get("title").map_err(db_error)?,
		author: row.try_get("author").map_err(db_error)?,
		cover_url: row.try_get("cover_url").map_err(db_error)?,
		series_name: row.try_get("series_name").map_err(db_error)?,
		series_sequence: row.try_get("series_sequence").map_err(db_error)?,
		library_path: row.try_get("library_path").map_err(db_error)?,
		merged_path: row.try_get("merged_path").map_err(db_error)?,
		merge_state: parse_enum::<MergeState>(&merge_state)?,
		merge_detail: row.try_get("merge_detail").map_err(db_error)?,
		merge_backup_path: row.try_get("merge_backup_path").map_err(db_error)?,
		source: parse_enum::<MediaSource>(&source)?,
		overridden: int_to_bool(overridden),
		matched_by: matched_by
			.as_deref()
			.map(parse_enum::<MatchedBy>)
			.transpose()?,
		request_id: row.try_get("request_id").map_err(db_error)?,
		created_at: parse_dt(&created_at)?,
	})
}

async fn insert_media(db: &Db, sql: &str, media: &Media) -> Result<()> {
	sqlx::query(sql)
		.bind(&media.id)
		.bind(media.work_id.as_deref())
		.bind(media.format.as_str())
		.bind(media.asin.as_deref())
		.bind(media.abs_item_id.as_deref())
		.bind(&media.title)
		.bind(media.author.as_deref())
		.bind(media.cover_url.as_deref())
		.bind(media.series_name.as_deref())
		.bind(media.series_sequence.as_deref())
		.bind(&media.library_path)
		.bind(media.merged_path.as_deref())
		.bind(media.merge_state.as_str())
		.bind(media.merge_detail.as_deref())
		.bind(media.merge_backup_path.as_deref())
		.bind(media.source.as_str())
		.bind(bool_to_int(media.overridden))
		.bind(media.matched_by.map(|m| m.as_str()))
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
			"INSERT INTO media ({COLUMNS}) VALUES ({}) \
			 ON CONFLICT (asin) DO UPDATE SET \
			 work_id = $2, title = $6, author = $7, cover_url = $8, library_path = $11, \
			 request_id = $19 \
			 WHERE media.overridden = 0",
			placeholders(1, COLUMN_COUNT)
		);
		insert_media(&self.db, &sql, media).await
	}

	async fn insert(&self, media: &Media) -> Result<()> {
		let sql = format!(
			"INSERT INTO media ({COLUMNS}) VALUES ({})",
			placeholders(1, COLUMN_COUNT)
		);
		insert_media(&self.db, &sql, media).await
	}

	async fn update(&self, media: &Media) -> Result<()> {
		sqlx::query(
			"UPDATE media SET \
			 work_id = $1, asin = $2, abs_item_id = $3, title = $4, author = $5, cover_url = $6, \
			 series_name = $7, series_sequence = $8, library_path = $9, \
			 overridden = $10, matched_by = $11 \
			 WHERE id = $12",
		)
		.bind(media.work_id.as_deref())
		.bind(media.asin.as_deref())
		.bind(media.abs_item_id.as_deref())
		.bind(&media.title)
		.bind(media.author.as_deref())
		.bind(media.cover_url.as_deref())
		.bind(media.series_name.as_deref())
		.bind(media.series_sequence.as_deref())
		.bind(&media.library_path)
		.bind(bool_to_int(media.overridden))
		.bind(media.matched_by.map(|m| m.as_str()))
		.bind(&media.id)
		.execute(&self.db)
		.await
		.map_err(map_write_error)?;
		Ok(())
	}

	async fn set_merge_state(
		&self,
		id: &str,
		state: MergeState,
		detail: Option<&str>,
	) -> Result<()> {
		sqlx::query("UPDATE media SET merge_state = $1, merge_detail = $2 WHERE id = $3")
			.bind(state.as_str())
			.bind(detail)
			.bind(id)
			.execute(&self.db)
			.await
			.map_err(map_write_error)?;
		Ok(())
	}

	async fn set_merge_result(
		&self,
		id: &str,
		merged_path: Option<&str>,
		merge_backup_path: Option<&str>,
		state: MergeState,
		detail: Option<&str>,
	) -> Result<()> {
		sqlx::query(
			"UPDATE media SET merged_path = $1, merge_backup_path = $2, \
			 merge_state = $3, merge_detail = $4 WHERE id = $5",
		)
		.bind(merged_path)
		.bind(merge_backup_path)
		.bind(state.as_str())
		.bind(detail)
		.bind(id)
		.execute(&self.db)
		.await
		.map_err(map_write_error)?;
		Ok(())
	}

	async fn delete(&self, id: &str) -> Result<()> {
		sqlx::query("DELETE FROM media WHERE id = $1")
			.bind(id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
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

	async fn find_by_request(&self, request_id: &str) -> Result<Option<Media>> {
		let row = sqlx::query("SELECT * FROM media WHERE request_id = $1 LIMIT 1")
			.bind(request_id)
			.fetch_optional(&self.db)
			.await
			.map_err(db_error)?;
		map_row_opt(row, map_media)
	}

	async fn available_among(&self, asins: &[String]) -> Result<Vec<String>> {
		if asins.is_empty() {
			return Ok(Vec::new());
		}
		let sql = format!(
			"SELECT asin FROM media WHERE asin IN ({})",
			placeholders(1, asins.len())
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

	async fn all(&self) -> Result<Vec<Media>> {
		let rows = sqlx::query("SELECT * FROM media")
			.fetch_all(&self.db)
			.await
			.map_err(db_error)?;
		map_rows(rows, map_media)
	}

	async fn list_page(&self, filter: MediaFilter, limit: i64, offset: i64) -> Result<Vec<Media>> {
		let sql = format!(
			"SELECT * FROM media {} ORDER BY created_at DESC LIMIT $1 OFFSET $2",
			list_filter(filter)
		);
		let rows = sqlx::query(&sql)
			.bind(limit)
			.bind(offset)
			.fetch_all(&self.db)
			.await
			.map_err(db_error)?;
		map_rows(rows, map_media)
	}

	async fn list_count(&self, filter: MediaFilter) -> Result<i64> {
		let sql = format!(
			"SELECT COUNT(*) AS count FROM media {}",
			list_filter(filter)
		);
		fetch_count(&self.db, sqlx::query(&sql)).await
	}

	async fn count(&self) -> Result<i64> {
		fetch_count(&self.db, sqlx::query("SELECT COUNT(*) AS count FROM media")).await
	}
}
