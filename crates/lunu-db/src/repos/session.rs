use async_trait::async_trait;
use chrono::{DateTime, Utc};
use lunu_core::Result;
use lunu_core::models::Session;
use lunu_core::repo::SessionRepo;
use sqlx::Row;
use sqlx::any::AnyRow;

use super::{map_row_opt, map_rows};
use crate::convert::{format_dt, parse_dt, parse_dt_opt};
use crate::{Db, db_error};

pub struct SqlxSessionRepo {
	db: Db,
}

impl SqlxSessionRepo {
	pub fn new(db: Db) -> Self {
		Self { db }
	}
}

fn map_session(row: &AnyRow) -> Result<Session> {
	let created_at: String = row.try_get("created_at").map_err(db_error)?;
	let expires_at: String = row.try_get("expires_at").map_err(db_error)?;
	let last_seen_at: Option<String> = row.try_get("last_seen_at").map_err(db_error)?;

	Ok(Session {
		id: row.try_get("id").map_err(db_error)?,
		user_id: row.try_get("user_id").map_err(db_error)?,
		token_hash: row.try_get("token_hash").map_err(db_error)?,
		created_at: parse_dt(&created_at)?,
		expires_at: parse_dt(&expires_at)?,
		last_seen_at: parse_dt_opt(last_seen_at)?,
		user_agent: row.try_get("user_agent").map_err(db_error)?,
	})
}

#[async_trait]
impl SessionRepo for SqlxSessionRepo {
	async fn create(&self, session: &Session) -> Result<()> {
		sqlx::query(
			"INSERT INTO sessions \
			 (id, user_id, token_hash, created_at, expires_at, last_seen_at, user_agent) \
			 VALUES ($1, $2, $3, $4, $5, $6, $7)",
		)
		.bind(&session.id)
		.bind(&session.user_id)
		.bind(&session.token_hash)
		.bind(format_dt(session.created_at))
		.bind(format_dt(session.expires_at))
		.bind(session.last_seen_at.map(format_dt))
		.bind(session.user_agent.as_deref())
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(())
	}

	async fn find_by_token_hash(&self, token_hash: &str) -> Result<Option<Session>> {
		let row = sqlx::query("SELECT * FROM sessions WHERE token_hash = $1")
			.bind(token_hash)
			.fetch_optional(&self.db)
			.await
			.map_err(db_error)?;
		map_row_opt(row, map_session)
	}

	async fn list_for_user(&self, user_id: &str) -> Result<Vec<Session>> {
		let rows =
			sqlx::query("SELECT * FROM sessions WHERE user_id = $1 ORDER BY created_at DESC")
				.bind(user_id)
				.fetch_all(&self.db)
				.await
				.map_err(db_error)?;
		map_rows(rows, map_session)
	}

	async fn touch(&self, id: &str, last_seen_at: DateTime<Utc>) -> Result<()> {
		sqlx::query("UPDATE sessions SET last_seen_at = $1 WHERE id = $2")
			.bind(format_dt(last_seen_at))
			.bind(id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}

	async fn set_user_agent(&self, id: &str, user_agent: &str) -> Result<()> {
		sqlx::query("UPDATE sessions SET user_agent = $1 WHERE id = $2")
			.bind(user_agent)
			.bind(id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}

	async fn delete(&self, id: &str) -> Result<()> {
		sqlx::query("DELETE FROM sessions WHERE id = $1")
			.bind(id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}

	async fn delete_for_user(&self, user_id: &str) -> Result<()> {
		sqlx::query("DELETE FROM sessions WHERE user_id = $1")
			.bind(user_id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}

	async fn delete_scoped(&self, user_id: &str, id: &str) -> Result<bool> {
		let result = sqlx::query("DELETE FROM sessions WHERE id = $1 AND user_id = $2")
			.bind(id)
			.bind(user_id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(result.rows_affected() > 0)
	}

	async fn delete_expired(&self, now: DateTime<Utc>) -> Result<()> {
		sqlx::query("DELETE FROM sessions WHERE expires_at <= $1")
			.bind(format_dt(now))
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}
}
