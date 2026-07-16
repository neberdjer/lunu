use std::str::FromStr;

use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::models::{ExternalId, IdScheme, Work, normalize};
use lunu_core::repo::WorkRepo;
use sqlx::Row;
use sqlx::any::AnyRow;

use super::{map_row_opt, map_rows};
use crate::convert::{format_dt, parse_dt};
use crate::{Db, db_error, map_write_error};

pub struct SqlxWorkRepo {
	db: Db,
}

impl SqlxWorkRepo {
	pub fn new(db: Db) -> Self {
		Self { db }
	}
}

fn map_work(row: &AnyRow) -> Result<Work> {
	let created_at: String = row.try_get("created_at").map_err(db_error)?;
	Ok(Work {
		id: row.try_get("id").map_err(db_error)?,
		title: row.try_get("title").map_err(db_error)?,
		author: row.try_get("author").map_err(db_error)?,
		cover_url: row.try_get("cover_url").map_err(db_error)?,
		created_at: parse_dt(&created_at)?,
	})
}

fn map_external_id(row: &AnyRow) -> Result<ExternalId> {
	let scheme: String = row.try_get("scheme").map_err(db_error)?;
	Ok(ExternalId {
		scheme: IdScheme::from_str(&scheme)?,
		value: row.try_get("value").map_err(db_error)?,
	})
}

#[async_trait]
impl WorkRepo for SqlxWorkRepo {
	async fn insert(&self, work: &Work) -> Result<()> {
		sqlx::query(
			"INSERT INTO works \
			 (id, title, author, normalized_title, normalized_author, cover_url, created_at) \
			 VALUES ($1, $2, $3, $4, $5, $6, $7)",
		)
		.bind(&work.id)
		.bind(&work.title)
		.bind(&work.author)
		.bind(normalize(&work.title))
		.bind(normalize(work.author.as_deref().unwrap_or_default()))
		.bind(&work.cover_url)
		.bind(format_dt(work.created_at))
		.execute(&self.db)
		.await
		.map_err(map_write_error)?;
		Ok(())
	}

	async fn find_by_id(&self, id: &str) -> Result<Option<Work>> {
		let row = sqlx::query("SELECT * FROM works WHERE id = $1")
			.bind(id)
			.fetch_optional(&self.db)
			.await
			.map_err(db_error)?;
		map_row_opt(row, map_work)
	}

	async fn find_by_external_id(&self, id: &ExternalId) -> Result<Option<String>> {
		sqlx::query_scalar("SELECT work_id FROM work_external_ids WHERE scheme = $1 AND value = $2")
			.bind(id.scheme.as_str())
			.bind(&id.value)
			.fetch_optional(&self.db)
			.await
			.map_err(db_error)
	}

	async fn find_unidentified_by_title(
		&self,
		title: &str,
		author: Option<&str>,
	) -> Result<Option<String>> {
		sqlx::query_scalar(
			"SELECT id FROM works \
			 WHERE normalized_title = $1 AND normalized_author = $2 \
			 AND NOT EXISTS (SELECT 1 FROM work_external_ids e WHERE e.work_id = works.id) \
			 ORDER BY created_at LIMIT 1",
		)
		.bind(normalize(title))
		.bind(normalize(author.unwrap_or_default()))
		.fetch_optional(&self.db)
		.await
		.map_err(db_error)
	}

	async fn link_external_id_if_absent(&self, work_id: &str, id: &ExternalId) -> Result<()> {
		sqlx::query(
			"INSERT INTO work_external_ids (scheme, value, work_id) VALUES ($1, $2, $3) \
			 ON CONFLICT (scheme, value) DO NOTHING",
		)
		.bind(id.scheme.as_str())
		.bind(&id.value)
		.bind(work_id)
		.execute(&self.db)
		.await
		.map_err(map_write_error)?;
		Ok(())
	}

	async fn link_external_id(&self, work_id: &str, id: &ExternalId) -> Result<()> {
		sqlx::query("INSERT INTO work_external_ids (scheme, value, work_id) VALUES ($1, $2, $3)")
			.bind(id.scheme.as_str())
			.bind(&id.value)
			.bind(work_id)
			.execute(&self.db)
			.await
			.map_err(map_write_error)?;
		Ok(())
	}

	async fn external_ids(&self, work_id: &str) -> Result<Vec<ExternalId>> {
		let rows = sqlx::query(
			"SELECT scheme, value FROM work_external_ids WHERE work_id = $1 ORDER BY scheme, value",
		)
		.bind(work_id)
		.fetch_all(&self.db)
		.await
		.map_err(db_error)?;
		map_rows(rows, map_external_id)
	}
}
