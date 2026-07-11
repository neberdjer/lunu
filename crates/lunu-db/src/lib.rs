use std::path::Path;

use lunu_core::{Error, Result};
use sqlx::AnyPool;
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

	AnyPoolOptions::new()
		.max_connections(DEFAULT_MAX_CONNECTIONS)
		.connect(database_url)
		.await
		.map_err(db_error)
}

pub async fn run_migrations(db: &Db) -> Result<()> {
	sqlx::migrate!("./migrations")
		.run(db)
		.await
		.map_err(db_error)
}

pub async fn ping(db: &Db) -> Result<()> {
	sqlx::query("SELECT 1")
		.execute(db)
		.await
		.map(|_| ())
		.map_err(db_error)
}

fn ensure_sqlite_parent(database_url: &str) -> Result<()> {
	let Some(rest) = database_url.strip_prefix("sqlite://") else {
		return Ok(());
	};

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
