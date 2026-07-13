use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::models::EmailVerificationToken;
use lunu_core::repo::EmailVerificationRepo;
use sqlx::Row;
use sqlx::any::AnyRow;

use super::map_row_opt;
use crate::convert::{format_dt, parse_dt};
use crate::{Db, db_error, map_write_error};

pub struct SqlxEmailVerificationRepo {
	db: Db,
}

impl SqlxEmailVerificationRepo {
	pub fn new(db: Db) -> Self {
		Self { db }
	}
}

fn map_token(row: &AnyRow) -> Result<EmailVerificationToken> {
	let created_at: String = row.try_get("created_at").map_err(db_error)?;
	let expires_at: String = row.try_get("expires_at").map_err(db_error)?;

	Ok(EmailVerificationToken {
		id: row.try_get("id").map_err(db_error)?,
		user_id: row.try_get("user_id").map_err(db_error)?,
		code_hash: row.try_get("code_hash").map_err(db_error)?,
		attempts: row.try_get("attempts").map_err(db_error)?,
		created_at: parse_dt(&created_at)?,
		expires_at: parse_dt(&expires_at)?,
	})
}

#[async_trait]
impl EmailVerificationRepo for SqlxEmailVerificationRepo {
	async fn create(&self, token: &EmailVerificationToken) -> Result<()> {
		sqlx::query(
			"INSERT INTO email_verification_tokens \
			 (id, user_id, code_hash, attempts, created_at, expires_at) \
			 VALUES ($1, $2, $3, $4, $5, $6)",
		)
		.bind(&token.id)
		.bind(&token.user_id)
		.bind(&token.code_hash)
		.bind(token.attempts)
		.bind(format_dt(token.created_at))
		.bind(format_dt(token.expires_at))
		.execute(&self.db)
		.await
		.map_err(map_write_error)?;
		Ok(())
	}

	async fn find_for_user(&self, user_id: &str) -> Result<Option<EmailVerificationToken>> {
		let row = sqlx::query("SELECT * FROM email_verification_tokens WHERE user_id = $1")
			.bind(user_id)
			.fetch_optional(&self.db)
			.await
			.map_err(db_error)?;
		map_row_opt(row, map_token)
	}

	async fn increment_attempts(&self, id: &str) -> Result<()> {
		sqlx::query("UPDATE email_verification_tokens SET attempts = attempts + 1 WHERE id = $1")
			.bind(id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}

	async fn delete(&self, id: &str) -> Result<()> {
		sqlx::query("DELETE FROM email_verification_tokens WHERE id = $1")
			.bind(id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}

	async fn delete_for_user(&self, user_id: &str) -> Result<()> {
		sqlx::query("DELETE FROM email_verification_tokens WHERE user_id = $1")
			.bind(user_id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}
}
