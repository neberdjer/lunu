use std::path::Path;

use lunu_core::helpers::matching::normalize;
use lunu_core::{Error, Result};
use sqlx::AnyPool;
use sqlx::Row;
use sqlx::any::{AnyPoolOptions, install_default_drivers};

mod convert;
pub mod repos;

#[cfg(test)]
mod service_tests;

pub type Db = AnyPool;

pub const DEFAULT_MAX_CONNECTIONS: u32 = 10;

pub(crate) fn db_error(error: impl std::fmt::Display) -> Error {
	Error::Database(error.to_string())
}

pub(crate) fn map_write_error(error: sqlx::Error) -> Error {
	if error
		.as_database_error()
		.is_some_and(|db| db.is_unique_violation())
	{
		return Error::Conflict(lunu_core::consts::reasons::ALREADY_EXISTS.to_string());
	}
	Error::Database(error.to_string())
}

pub async fn connect(database_url: &str) -> Result<Db> {
	install_default_drivers();
	ensure_sqlite_parent(database_url)?;

	let is_sqlite = is_sqlite_url(database_url);
	AnyPoolOptions::new()
		.max_connections(DEFAULT_MAX_CONNECTIONS)
		.after_connect(move |conn, _meta| {
			Box::pin(async move {
				if is_sqlite {
					apply_sqlite_pragmas(conn).await?;
				}
				Ok(())
			})
		})
		.connect(database_url)
		.await
		.map_err(db_error)
}

async fn apply_sqlite_pragmas(conn: &mut sqlx::AnyConnection) -> sqlx::Result<()> {
	let mode: Option<String> = sqlx::query_scalar("PRAGMA journal_mode = WAL")
		.fetch_optional(&mut *conn)
		.await?;

	if !mode
		.as_deref()
		.is_some_and(|mode| mode.eq_ignore_ascii_case("wal"))
	{
		tracing::warn!(
			?mode,
			"sqlite is not in WAL mode, so writers will block readers; WAL needs a local filesystem and does not work over NFS or SMB"
		);
	}

	sqlx::query("PRAGMA synchronous = NORMAL")
		.execute(&mut *conn)
		.await?;
	Ok(())
}

pub async fn run_migrations(db: &Db) -> Result<()> {
	sqlx::migrate!("./migrations")
		.run(db)
		.await
		.map_err(db_error)?;
	normalize_works(db).await
}

async fn normalize_works(db: &Db) -> Result<()> {
	let rows = sqlx::query("SELECT id, title, author FROM works WHERE normalized_title IS NULL")
		.fetch_all(db)
		.await
		.map_err(db_error)?;

	for row in &rows {
		let id: String = row.try_get("id").map_err(db_error)?;
		let title: String = row.try_get("title").map_err(db_error)?;
		let author: Option<String> = row.try_get("author").map_err(db_error)?;

		sqlx::query("UPDATE works SET normalized_title = $1, normalized_author = $2 WHERE id = $3")
			.bind(normalize(&title))
			.bind(normalize(author.as_deref().unwrap_or_default()))
			.bind(&id)
			.execute(db)
			.await
			.map_err(db_error)?;
	}

	Ok(())
}

pub async fn ping(db: &Db) -> Result<()> {
	sqlx::query("SELECT 1")
		.execute(db)
		.await
		.map(|_| ())
		.map_err(db_error)
}

fn is_sqlite_url(database_url: &str) -> bool {
	database_url.starts_with("sqlite:")
}

fn ensure_sqlite_parent(database_url: &str) -> Result<()> {
	if !is_sqlite_url(database_url) {
		return Ok(());
	}
	let rest = database_url
		.strip_prefix("sqlite://")
		.or_else(|| database_url.strip_prefix("sqlite:"))
		.unwrap_or_default();

	let path_part = rest.split('?').next().unwrap_or(rest);
	if path_part.is_empty() || path_part == ":memory:" {
		return Ok(());
	}

	if let Some(parent) = Path::new(path_part).parent()
		&& !parent.as_os_str().is_empty()
	{
		std::fs::create_dir_all(parent).map_err(db_error)?;
	}

	Ok(())
}
