use async_trait::async_trait;
use chrono::{DateTime, Utc};
use lunu_core::Result;
use lunu_core::models::ApiKey;
use lunu_core::repo::ApiKeyRepo;
use sqlx::Row;
use sqlx::any::AnyRow;

use super::{fetch_count, map_row_opt, map_rows};
use crate::convert::{
	bool_to_int, format_dt, int_to_bool, join_list, parse_dt, parse_dt_opt, split_list,
};
use crate::{Db, db_error};

pub struct SqlxApiKeyRepo {
	db: Db,
}

impl SqlxApiKeyRepo {
	pub fn new(db: Db) -> Self {
		Self { db }
	}
}

fn map_api_key(row: &AnyRow) -> Result<ApiKey> {
	let scopes: String = row.try_get("scopes").map_err(db_error)?;
	let created_at: String = row.try_get("created_at").map_err(db_error)?;
	let last_used_at: Option<String> = row.try_get("last_used_at").map_err(db_error)?;
	let expires_at: Option<String> = row.try_get("expires_at").map_err(db_error)?;
	let revoked: i64 = row.try_get("revoked").map_err(db_error)?;

	Ok(ApiKey {
		id: row.try_get("id").map_err(db_error)?,
		user_id: row.try_get("user_id").map_err(db_error)?,
		name: row.try_get("name").map_err(db_error)?,
		prefix: row.try_get("prefix").map_err(db_error)?,
		key_hash: row.try_get("key_hash").map_err(db_error)?,
		scopes: split_list(&scopes),
		created_at: parse_dt(&created_at)?,
		last_used_at: parse_dt_opt(last_used_at)?,
		expires_at: parse_dt_opt(expires_at)?,
		revoked: int_to_bool(revoked),
	})
}

#[async_trait]
impl ApiKeyRepo for SqlxApiKeyRepo {
	async fn create(&self, key: &ApiKey) -> Result<()> {
		sqlx::query(
			"INSERT INTO api_keys \
			 (id, user_id, name, prefix, key_hash, scopes, created_at, last_used_at, expires_at, revoked) \
			 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
		)
		.bind(&key.id)
		.bind(&key.user_id)
		.bind(&key.name)
		.bind(&key.prefix)
		.bind(&key.key_hash)
		.bind(join_list(&key.scopes))
		.bind(format_dt(key.created_at))
		.bind(key.last_used_at.map(format_dt))
		.bind(key.expires_at.map(format_dt))
		.bind(bool_to_int(key.revoked))
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(())
	}

	async fn find_by_key_hash(&self, key_hash: &str) -> Result<Option<ApiKey>> {
		let row = sqlx::query("SELECT * FROM api_keys WHERE key_hash = $1")
			.bind(key_hash)
			.fetch_optional(&self.db)
			.await
			.map_err(db_error)?;
		map_row_opt(row, map_api_key)
	}

	async fn list_for_user(&self, user_id: &str) -> Result<Vec<ApiKey>> {
		let rows = sqlx::query("SELECT * FROM api_keys WHERE user_id = $1 ORDER BY created_at")
			.bind(user_id)
			.fetch_all(&self.db)
			.await
			.map_err(db_error)?;
		map_rows(rows, map_api_key)
	}

	async fn list_for_user_page(
		&self,
		user_id: &str,
		limit: i64,
		offset: i64,
	) -> Result<Vec<ApiKey>> {
		let rows = sqlx::query(
			"SELECT * FROM api_keys WHERE user_id = $1 ORDER BY created_at LIMIT $2 OFFSET $3",
		)
		.bind(user_id)
		.bind(limit)
		.bind(offset)
		.fetch_all(&self.db)
		.await
		.map_err(db_error)?;
		map_rows(rows, map_api_key)
	}

	async fn count_for_user(&self, user_id: &str) -> Result<i64> {
		fetch_count(
			&self.db,
			sqlx::query("SELECT COUNT(*) AS count FROM api_keys WHERE user_id = $1").bind(user_id),
		)
		.await
	}

	async fn touch_last_used(&self, id: &str, at: DateTime<Utc>) -> Result<()> {
		sqlx::query("UPDATE api_keys SET last_used_at = $1 WHERE id = $2")
			.bind(format_dt(at))
			.bind(id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}

	async fn set_revoked(&self, id: &str, revoked: bool) -> Result<()> {
		sqlx::query("UPDATE api_keys SET revoked = $1 WHERE id = $2")
			.bind(bool_to_int(revoked))
			.bind(id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}

	async fn revoke_owned(&self, id: &str, user_id: &str) -> Result<bool> {
		let result = sqlx::query("UPDATE api_keys SET revoked = $1 WHERE id = $2 AND user_id = $3")
			.bind(bool_to_int(true))
			.bind(id)
			.bind(user_id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(result.rows_affected() > 0)
	}

	async fn delete(&self, id: &str) -> Result<()> {
		sqlx::query("DELETE FROM api_keys WHERE id = $1")
			.bind(id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}
}
