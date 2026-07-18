use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::models::{MfaMethod, UserMfa};
use lunu_core::repo::UserMfaRepo;
use sqlx::Row;
use sqlx::any::AnyRow;

use super::map_row_opt;
use crate::convert::{bool_to_int, format_dt, int_to_bool, parse_dt, parse_enum};
use crate::{Db, db_error, map_write_error};

pub struct SqlxUserMfaRepo {
	db: Db,
}

impl SqlxUserMfaRepo {
	pub fn new(db: Db) -> Self {
		Self { db }
	}
}

fn map_mfa(row: &AnyRow) -> Result<UserMfa> {
	let method: String = row.try_get("method").map_err(db_error)?;
	let confirmed: i64 = row.try_get("confirmed").map_err(db_error)?;
	let created_at: String = row.try_get("created_at").map_err(db_error)?;
	let updated_at: String = row.try_get("updated_at").map_err(db_error)?;

	Ok(UserMfa {
		user_id: row.try_get("user_id").map_err(db_error)?,
		method: parse_enum::<MfaMethod>(&method)?,
		secret: row.try_get("secret").map_err(db_error)?,
		confirmed: int_to_bool(confirmed),
		created_at: parse_dt(&created_at)?,
		updated_at: parse_dt(&updated_at)?,
	})
}

#[async_trait]
impl UserMfaRepo for SqlxUserMfaRepo {
	async fn upsert(&self, mfa: &UserMfa) -> Result<()> {
		sqlx::query(
			"INSERT INTO user_mfa (user_id, method, secret, confirmed, created_at, updated_at) \
			 VALUES ($1, $2, $3, $4, $5, $6) \
			 ON CONFLICT (user_id) DO UPDATE SET \
			 method = $2, secret = $3, confirmed = $4, updated_at = $6",
		)
		.bind(&mfa.user_id)
		.bind(mfa.method.as_str())
		.bind(mfa.secret.as_deref())
		.bind(bool_to_int(mfa.confirmed))
		.bind(format_dt(mfa.created_at))
		.bind(format_dt(mfa.updated_at))
		.execute(&self.db)
		.await
		.map_err(map_write_error)?;
		Ok(())
	}

	async fn find_for_user(&self, user_id: &str) -> Result<Option<UserMfa>> {
		let row = sqlx::query("SELECT * FROM user_mfa WHERE user_id = $1")
			.bind(user_id)
			.fetch_optional(&self.db)
			.await
			.map_err(db_error)?;
		map_row_opt(row, map_mfa)
	}

	async fn delete_for_user(&self, user_id: &str) -> Result<()> {
		sqlx::query("DELETE FROM user_mfa WHERE user_id = $1")
			.bind(user_id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}
}
