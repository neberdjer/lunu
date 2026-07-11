use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::models::Setting;
use lunu_core::repo::SettingsRepo;
use sqlx::Row;
use sqlx::any::AnyRow;

use super::{map_row_opt, map_rows};
use crate::convert::{bool_to_int, format_dt, int_to_bool, parse_dt};
use crate::{Db, db_error};

pub struct SqlxSettingsRepo {
	db: Db,
}

impl SqlxSettingsRepo {
	pub fn new(db: Db) -> Self {
		Self { db }
	}
}

fn map_setting(row: &AnyRow) -> Result<Setting> {
	let encrypted: i64 = row.try_get("encrypted").map_err(db_error)?;
	let updated_at: String = row.try_get("updated_at").map_err(db_error)?;

	Ok(Setting {
		key: row.try_get("key").map_err(db_error)?,
		value: row.try_get("value").map_err(db_error)?,
		encrypted: int_to_bool(encrypted),
		updated_at: parse_dt(&updated_at)?,
	})
}

#[async_trait]
impl SettingsRepo for SqlxSettingsRepo {
	async fn get(&self, key: &str) -> Result<Option<Setting>> {
		let row = sqlx::query("SELECT * FROM settings WHERE key = $1")
			.bind(key)
			.fetch_optional(&self.db)
			.await
			.map_err(db_error)?;
		map_row_opt(row, map_setting)
	}

	async fn set(&self, setting: &Setting) -> Result<()> {
		sqlx::query(
			"INSERT INTO settings (key, value, encrypted, updated_at) VALUES ($1, $2, $3, $4) \
			 ON CONFLICT(key) DO UPDATE SET \
			 value = excluded.value, encrypted = excluded.encrypted, updated_at = excluded.updated_at",
		)
		.bind(&setting.key)
		.bind(&setting.value)
		.bind(bool_to_int(setting.encrypted))
		.bind(format_dt(setting.updated_at))
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(())
	}

	async fn get_all(&self) -> Result<Vec<Setting>> {
		let rows = sqlx::query("SELECT * FROM settings ORDER BY key")
			.fetch_all(&self.db)
			.await
			.map_err(db_error)?;
		map_rows(rows, map_setting)
	}

	async fn delete(&self, key: &str) -> Result<()> {
		sqlx::query("DELETE FROM settings WHERE key = $1")
			.bind(key)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}
}
