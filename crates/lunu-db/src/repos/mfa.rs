use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::models::{MfaMethod, MfaRecoveryCode, UserMfa};
use lunu_core::repo::{MfaRecoveryCodeRepo, UserMfaRepo};
use sqlx::Row;
use sqlx::any::AnyRow;

use super::{fetch_count, map_row_opt};
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
	let last_totp_step: i64 = row.try_get("last_totp_step").map_err(db_error)?;
	let created_at: String = row.try_get("created_at").map_err(db_error)?;
	let updated_at: String = row.try_get("updated_at").map_err(db_error)?;

	Ok(UserMfa {
		user_id: row.try_get("user_id").map_err(db_error)?,
		method: parse_enum::<MfaMethod>(&method)?,
		secret: row.try_get("secret").map_err(db_error)?,
		confirmed: int_to_bool(confirmed),
		last_totp_step,
		created_at: parse_dt(&created_at)?,
		updated_at: parse_dt(&updated_at)?,
	})
}

#[async_trait]
impl UserMfaRepo for SqlxUserMfaRepo {
	async fn record_totp_step(&self, user_id: &str, step: i64) -> Result<()> {
		sqlx::query(
			"UPDATE user_mfa SET last_totp_step = $1 WHERE user_id = $2 AND last_totp_step < $1",
		)
		.bind(step)
		.bind(user_id)
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(())
	}

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

pub struct SqlxMfaRecoveryCodeRepo {
	db: Db,
}

impl SqlxMfaRecoveryCodeRepo {
	pub fn new(db: Db) -> Self {
		Self { db }
	}
}

#[async_trait]
impl MfaRecoveryCodeRepo for SqlxMfaRecoveryCodeRepo {
	async fn replace_for_user(&self, user_id: &str, codes: &[MfaRecoveryCode]) -> Result<()> {
		self.delete_for_user(user_id).await?;
		for code in codes {
			sqlx::query(
				"INSERT INTO mfa_recovery_codes (id, user_id, code_hash, used_at, created_at) \
				 VALUES ($1, $2, $3, NULL, $4)",
			)
			.bind(&code.id)
			.bind(&code.user_id)
			.bind(&code.code_hash)
			.bind(format_dt(code.created_at))
			.execute(&self.db)
			.await
			.map_err(map_write_error)?;
		}
		Ok(())
	}

	async fn consume(&self, user_id: &str, code_hash: &str) -> Result<bool> {
		let result = sqlx::query(
			"UPDATE mfa_recovery_codes SET used_at = $1 \
			 WHERE user_id = $2 AND code_hash = $3 AND used_at IS NULL",
		)
		.bind(format_dt(chrono::Utc::now()))
		.bind(user_id)
		.bind(code_hash)
		.execute(&self.db)
		.await
		.map_err(map_write_error)?;
		Ok(result.rows_affected() > 0)
	}

	async fn count_unused(&self, user_id: &str) -> Result<i64> {
		fetch_count(
			&self.db,
			sqlx::query(
				"SELECT COUNT(*) AS count FROM mfa_recovery_codes \
				 WHERE user_id = $1 AND used_at IS NULL",
			)
			.bind(user_id),
		)
		.await
	}

	async fn delete_for_user(&self, user_id: &str) -> Result<()> {
		sqlx::query("DELETE FROM mfa_recovery_codes WHERE user_id = $1")
			.bind(user_id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}
}
