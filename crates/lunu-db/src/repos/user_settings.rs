use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::models::UserSettings;
use lunu_core::repo::UserSettingsRepo;
use sqlx::Row;
use sqlx::any::AnyRow;

use super::map_row_opt;
use crate::convert::{bool_to_int, format_dt, int_to_bool, parse_dt};
use crate::{Db, db_error};

pub struct SqlxUserSettingsRepo {
	db: Db,
}

impl SqlxUserSettingsRepo {
	pub fn new(db: Db) -> Self {
		Self { db }
	}
}

fn map_settings(row: &AnyRow) -> Result<UserSettings> {
	let auto_approve: i64 = row.try_get("auto_approve").map_err(db_error)?;
	let updated_at: String = row.try_get("updated_at").map_err(db_error)?;

	Ok(UserSettings {
		user_id: row.try_get("user_id").map_err(db_error)?,
		auto_approve: int_to_bool(auto_approve),
		request_quota: row.try_get("request_quota").map_err(db_error)?,
		quota_days: row.try_get("quota_days").map_err(db_error)?,
		updated_at: parse_dt(&updated_at)?,
	})
}

#[async_trait]
impl UserSettingsRepo for SqlxUserSettingsRepo {
	async fn get(&self, user_id: &str) -> Result<Option<UserSettings>> {
		let row = sqlx::query("SELECT * FROM user_settings WHERE user_id = ?")
			.bind(user_id)
			.fetch_optional(&self.db)
			.await
			.map_err(db_error)?;
		map_row_opt(row, map_settings)
	}

	async fn upsert(&self, settings: &UserSettings) -> Result<()> {
		sqlx::query(
			"INSERT INTO user_settings \
			 (user_id, auto_approve, request_quota, quota_days, updated_at) \
			 VALUES (?, ?, ?, ?, ?) \
			 ON CONFLICT(user_id) DO UPDATE SET \
			 auto_approve = excluded.auto_approve, request_quota = excluded.request_quota, \
			 quota_days = excluded.quota_days, updated_at = excluded.updated_at",
		)
		.bind(&settings.user_id)
		.bind(bool_to_int(settings.auto_approve))
		.bind(settings.request_quota)
		.bind(settings.quota_days)
		.bind(format_dt(settings.updated_at))
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(())
	}

	async fn delete(&self, user_id: &str) -> Result<()> {
		sqlx::query("DELETE FROM user_settings WHERE user_id = ?")
			.bind(user_id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}
}
