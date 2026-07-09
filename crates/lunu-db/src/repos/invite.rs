use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::models::{Invite, Role};
use lunu_core::repo::InviteRepo;
use sqlx::Row;
use sqlx::any::AnyRow;

use super::{map_row_opt, map_rows};
use crate::convert::{format_dt, parse_dt, parse_dt_opt, parse_enum};
use crate::{Db, db_error};

pub struct SqlxInviteRepo {
	db: Db,
}

impl SqlxInviteRepo {
	pub fn new(db: Db) -> Self {
		Self { db }
	}
}

fn map_invite(row: &AnyRow) -> Result<Invite> {
	let role: String = row.try_get("role").map_err(db_error)?;
	let created_at: String = row.try_get("created_at").map_err(db_error)?;
	let expires_at: Option<String> = row.try_get("expires_at").map_err(db_error)?;

	Ok(Invite {
		id: row.try_get("id").map_err(db_error)?,
		code_hash: row.try_get("code_hash").map_err(db_error)?,
		role: parse_enum::<Role>(&role)?,
		email: row.try_get("email").map_err(db_error)?,
		created_by: row.try_get("created_by").map_err(db_error)?,
		max_uses: row.try_get("max_uses").map_err(db_error)?,
		used_count: row.try_get("used_count").map_err(db_error)?,
		created_at: parse_dt(&created_at)?,
		expires_at: parse_dt_opt(expires_at)?,
	})
}

#[async_trait]
impl InviteRepo for SqlxInviteRepo {
	async fn create(&self, invite: &Invite) -> Result<()> {
		sqlx::query(
			"INSERT INTO invites \
			 (id, code_hash, role, email, created_by, max_uses, used_count, created_at, expires_at) \
			 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
		)
		.bind(&invite.id)
		.bind(&invite.code_hash)
		.bind(invite.role.as_str())
		.bind(invite.email.as_deref())
		.bind(&invite.created_by)
		.bind(invite.max_uses)
		.bind(invite.used_count)
		.bind(format_dt(invite.created_at))
		.bind(invite.expires_at.map(format_dt))
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(())
	}

	async fn find_by_code_hash(&self, code_hash: &str) -> Result<Option<Invite>> {
		let row = sqlx::query("SELECT * FROM invites WHERE code_hash = ?")
			.bind(code_hash)
			.fetch_optional(&self.db)
			.await
			.map_err(db_error)?;
		map_row_opt(row, map_invite)
	}

	async fn increment_used(&self, id: &str) -> Result<()> {
		sqlx::query("UPDATE invites SET used_count = used_count + 1 WHERE id = ?")
			.bind(id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}

	async fn list(&self) -> Result<Vec<Invite>> {
		let rows = sqlx::query("SELECT * FROM invites ORDER BY created_at")
			.fetch_all(&self.db)
			.await
			.map_err(db_error)?;
		map_rows(rows, map_invite)
	}

	async fn delete(&self, id: &str) -> Result<()> {
		sqlx::query("DELETE FROM invites WHERE id = ?")
			.bind(id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}
}
