use async_trait::async_trait;
use chrono::{DateTime, Utc};
use lunu_core::Result;
use lunu_core::models::Session;
use lunu_core::repo::SessionRepo;
use sqlx::Row;
use sqlx::any::AnyRow;

use super::map_row_opt;
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
			 VALUES (?, ?, ?, ?, ?, ?, ?)",
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
		let row = sqlx::query("SELECT * FROM sessions WHERE token_hash = ?")
			.bind(token_hash)
			.fetch_optional(&self.db)
			.await
			.map_err(db_error)?;
		map_row_opt(row, map_session)
	}

	async fn touch(&self, id: &str, last_seen_at: DateTime<Utc>) -> Result<()> {
		sqlx::query("UPDATE sessions SET last_seen_at = ? WHERE id = ?")
			.bind(format_dt(last_seen_at))
			.bind(id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}

	async fn delete(&self, id: &str) -> Result<()> {
		sqlx::query("DELETE FROM sessions WHERE id = ?")
			.bind(id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}

	async fn delete_for_user(&self, user_id: &str) -> Result<()> {
		sqlx::query("DELETE FROM sessions WHERE user_id = ?")
			.bind(user_id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}

	async fn delete_expired(&self, now: DateTime<Utc>) -> Result<()> {
		sqlx::query("DELETE FROM sessions WHERE expires_at <= ?")
			.bind(format_dt(now))
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}
}
