use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::models::{AuthSource, Role, User};
use lunu_core::repo::UserRepo;
use sqlx::Row;
use sqlx::any::AnyRow;

use super::{fetch_count, map_row_opt, map_rows};
use crate::convert::{bool_to_int, format_dt, int_to_bool, parse_dt, parse_enum};
use crate::{Db, db_error, map_write_error};

pub struct SqlxUserRepo {
	db: Db,
}

impl SqlxUserRepo {
	pub fn new(db: Db) -> Self {
		Self { db }
	}
}

fn map_user(row: &AnyRow) -> Result<User> {
	let role: String = row.try_get("role").map_err(db_error)?;
	let auth_source: String = row.try_get("auth_source").map_err(db_error)?;
	let created_at: String = row.try_get("created_at").map_err(db_error)?;
	let updated_at: String = row.try_get("updated_at").map_err(db_error)?;
	let enabled: i64 = row.try_get("enabled").map_err(db_error)?;
	let email_verified: i64 = row.try_get("email_verified").map_err(db_error)?;

	Ok(User {
		id: row.try_get("id").map_err(db_error)?,
		username: row.try_get("username").map_err(db_error)?,
		email: row.try_get("email").map_err(db_error)?,
		display_name: row.try_get("display_name").map_err(db_error)?,
		locale: row.try_get("locale").map_err(db_error)?,
		password_hash: row.try_get("password_hash").map_err(db_error)?,
		role: parse_enum::<Role>(&role)?,
		auth_source: parse_enum::<AuthSource>(&auth_source)?,
		enabled: int_to_bool(enabled),
		email_verified: int_to_bool(email_verified),
		created_at: parse_dt(&created_at)?,
		updated_at: parse_dt(&updated_at)?,
	})
}

#[async_trait]
impl UserRepo for SqlxUserRepo {
	async fn create(&self, user: &User) -> Result<()> {
		sqlx::query(
			"INSERT INTO users \
			 (id, username, email, display_name, locale, password_hash, role, auth_source, enabled, email_verified, created_at, updated_at) \
			 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
		)
		.bind(&user.id)
		.bind(&user.username)
		.bind(user.email.as_deref())
		.bind(user.display_name.as_deref())
		.bind(user.locale.as_deref())
		.bind(user.password_hash.as_deref())
		.bind(user.role.as_str())
		.bind(user.auth_source.as_str())
		.bind(bool_to_int(user.enabled))
		.bind(bool_to_int(user.email_verified))
		.bind(format_dt(user.created_at))
		.bind(format_dt(user.updated_at))
		.execute(&self.db)
		.await
		.map_err(map_write_error)?;
		Ok(())
	}

	async fn create_initial_admin(&self, user: &User) -> Result<bool> {
		let result = sqlx::query(
			"INSERT INTO users \
			 (id, username, email, display_name, locale, password_hash, role, auth_source, enabled, email_verified, created_at, updated_at) \
			 SELECT $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12 \
			 WHERE NOT EXISTS (SELECT 1 FROM users)",
		)
		.bind(&user.id)
		.bind(&user.username)
		.bind(user.email.as_deref())
		.bind(user.display_name.as_deref())
		.bind(user.locale.as_deref())
		.bind(user.password_hash.as_deref())
		.bind(user.role.as_str())
		.bind(user.auth_source.as_str())
		.bind(bool_to_int(user.enabled))
		.bind(bool_to_int(user.email_verified))
		.bind(format_dt(user.created_at))
		.bind(format_dt(user.updated_at))
		.execute(&self.db)
		.await
		.map_err(map_write_error)?;
		Ok(result.rows_affected() > 0)
	}

	async fn count_enabled_admins_excluding(&self, id: &str) -> Result<i64> {
		fetch_count(
			&self.db,
			sqlx::query(
				"SELECT COUNT(*) AS count FROM users \
				 WHERE role = $1 AND enabled = $2 AND id <> $3",
			)
			.bind(Role::Admin.as_str())
			.bind(bool_to_int(true))
			.bind(id),
		)
		.await
	}

	async fn update(&self, user: &User) -> Result<()> {
		sqlx::query(
			"UPDATE users SET \
			 username = $1, email = $2, display_name = $3, locale = $4, password_hash = $5, \
			 role = $6, auth_source = $7, enabled = $8, email_verified = $9, updated_at = $10 WHERE id = $11",
		)
		.bind(&user.username)
		.bind(user.email.as_deref())
		.bind(user.display_name.as_deref())
		.bind(user.locale.as_deref())
		.bind(user.password_hash.as_deref())
		.bind(user.role.as_str())
		.bind(user.auth_source.as_str())
		.bind(bool_to_int(user.enabled))
		.bind(bool_to_int(user.email_verified))
		.bind(format_dt(user.updated_at))
		.bind(&user.id)
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(())
	}

	async fn mark_email_verified(&self, id: &str) -> Result<()> {
		sqlx::query("UPDATE users SET email_verified = $1 WHERE id = $2")
			.bind(bool_to_int(true))
			.bind(id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}

	async fn find_by_id(&self, id: &str) -> Result<Option<User>> {
		let row = sqlx::query("SELECT * FROM users WHERE id = $1")
			.bind(id)
			.fetch_optional(&self.db)
			.await
			.map_err(db_error)?;
		map_row_opt(row, map_user)
	}

	async fn find_by_username(&self, username: &str) -> Result<Option<User>> {
		let row = sqlx::query("SELECT * FROM users WHERE username = $1")
			.bind(username)
			.fetch_optional(&self.db)
			.await
			.map_err(db_error)?;
		map_row_opt(row, map_user)
	}

	async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
		let row = sqlx::query("SELECT * FROM users WHERE email = $1")
			.bind(email)
			.fetch_optional(&self.db)
			.await
			.map_err(db_error)?;
		map_row_opt(row, map_user)
	}

	async fn list(&self) -> Result<Vec<User>> {
		let rows = sqlx::query("SELECT * FROM users ORDER BY created_at")
			.fetch_all(&self.db)
			.await
			.map_err(db_error)?;
		map_rows(rows, map_user)
	}

	async fn enabled_admin_ids(&self) -> Result<Vec<String>> {
		let rows = sqlx::query("SELECT id FROM users WHERE role = $1 AND enabled = $2")
			.bind(Role::Admin.as_str())
			.bind(bool_to_int(true))
			.fetch_all(&self.db)
			.await
			.map_err(db_error)?;
		rows.iter()
			.map(|row| row.try_get("id").map_err(db_error))
			.collect()
	}

	async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<User>> {
		let rows = sqlx::query("SELECT * FROM users ORDER BY created_at LIMIT $1 OFFSET $2")
			.bind(limit)
			.bind(offset)
			.fetch_all(&self.db)
			.await
			.map_err(db_error)?;
		map_rows(rows, map_user)
	}

	async fn count(&self) -> Result<i64> {
		fetch_count(&self.db, sqlx::query("SELECT COUNT(*) AS count FROM users")).await
	}

	async fn delete(&self, id: &str) -> Result<()> {
		sqlx::query("DELETE FROM users WHERE id = $1")
			.bind(id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use chrono::Utc;
	use sqlx::any::{AnyPoolOptions, install_default_drivers};

	use super::*;
	use crate::run_migrations;

	async fn memory_repo() -> SqlxUserRepo {
		install_default_drivers();
		let db = AnyPoolOptions::new()
			.max_connections(1)
			.connect("sqlite::memory:")
			.await
			.unwrap();
		run_migrations(&db).await.unwrap();
		SqlxUserRepo::new(db)
	}

	fn sample_user() -> User {
		let now = Utc::now();
		User {
			id: "user-1".to_string(),
			username: "alice".to_string(),
			email: Some("alice@example.com".to_string()),
			display_name: None,
			locale: None,
			password_hash: Some("hash".to_string()),
			role: Role::Admin,
			auth_source: AuthSource::Local,
			enabled: true,
			email_verified: true,
			created_at: now,
			updated_at: now,
		}
	}

	#[tokio::test]
	async fn create_count_and_fetch() {
		let repo = memory_repo().await;
		assert_eq!(repo.count().await.unwrap(), 0);

		repo.create(&sample_user()).await.unwrap();
		assert_eq!(repo.count().await.unwrap(), 1);

		let fetched = repo.find_by_username("alice").await.unwrap().unwrap();
		assert_eq!(fetched.id, "user-1");
		assert_eq!(fetched.role, Role::Admin);
		assert_eq!(fetched.auth_source, AuthSource::Local);
		assert!(fetched.enabled);
		assert_eq!(fetched.email.as_deref(), Some("alice@example.com"));

		assert!(repo.find_by_username("nobody").await.unwrap().is_none());
	}

	#[tokio::test]
	async fn update_and_delete() {
		let repo = memory_repo().await;
		repo.create(&sample_user()).await.unwrap();

		let mut user = repo.find_by_id("user-1").await.unwrap().unwrap();
		user.enabled = false;
		user.role = Role::User;
		repo.update(&user).await.unwrap();

		let updated = repo.find_by_id("user-1").await.unwrap().unwrap();
		assert!(!updated.enabled);
		assert_eq!(updated.role, Role::User);

		repo.delete("user-1").await.unwrap();
		assert_eq!(repo.count().await.unwrap(), 0);
	}
}
