mod activity;
mod api_key;
mod blocklist;
mod download;
mod invite;
mod issue;
mod job;
mod media;
mod metadata_cache;
mod quality_profile;
mod request;
mod session;
mod settings;
mod user;
mod user_notification;
mod user_settings;

pub use activity::SqlxActivityRepo;
pub use api_key::SqlxApiKeyRepo;
pub use blocklist::SqlxBlocklistRepo;
pub use download::SqlxDownloadRepo;
pub use invite::SqlxInviteRepo;
pub use issue::SqlxIssueRepo;
pub use job::SqlxJobRepo;
pub use media::SqlxMediaRepo;
pub use metadata_cache::SqlxMetadataCacheRepo;
pub use quality_profile::SqlxQualityProfileRepo;
pub use request::SqlxRequestRepo;
pub use session::SqlxSessionRepo;
pub use settings::SqlxSettingsRepo;
pub use user::SqlxUserRepo;
pub use user_notification::SqlxUserNotificationRepo;
pub use user_settings::SqlxUserSettingsRepo;

use lunu_core::Result;
use sqlx::Row;
use sqlx::any::{AnyArguments, AnyRow};
use sqlx::query::Query;

use crate::{Db, db_error};

pub(crate) fn map_row_opt<T>(
	row: Option<AnyRow>,
	map: fn(&AnyRow) -> Result<T>,
) -> Result<Option<T>> {
	row.as_ref().map(map).transpose()
}

pub(crate) fn map_rows<T>(rows: Vec<AnyRow>, map: fn(&AnyRow) -> Result<T>) -> Result<Vec<T>> {
	rows.iter().map(map).collect()
}

pub(crate) async fn fetch_count<'q>(
	db: &Db,
	query: Query<'q, sqlx::Any, AnyArguments<'q>>,
) -> Result<i64> {
	let row = query.fetch_one(db).await.map_err(db_error)?;
	row.try_get::<i64, _>("count").map_err(db_error)
}

fn status_clause(status: Option<&str>) -> (&'static str, i64) {
	match status {
		Some(_) => (" WHERE status = $1", 2),
		None => ("", 1),
	}
}

pub(crate) async fn list_by_status<T>(
	db: &Db,
	table: &str,
	status: Option<&str>,
	limit: i64,
	offset: i64,
	map: fn(&AnyRow) -> Result<T>,
) -> Result<Vec<T>> {
	let (clause, next) = status_clause(status);
	let sql = format!(
		"SELECT * FROM {table}{clause} ORDER BY created_at DESC LIMIT ${next} OFFSET ${}",
		next + 1
	);
	let mut query = sqlx::query(&sql);
	if let Some(status) = status {
		query = query.bind(status);
	}
	let rows = query
		.bind(limit)
		.bind(offset)
		.fetch_all(db)
		.await
		.map_err(db_error)?;
	map_rows(rows, map)
}

pub(crate) async fn count_by_status(db: &Db, table: &str, status: Option<&str>) -> Result<i64> {
	let (clause, _) = status_clause(status);
	let sql = format!("SELECT COUNT(*) AS count FROM {table}{clause}");
	let mut query = sqlx::query(&sql);
	if let Some(status) = status {
		query = query.bind(status);
	}
	fetch_count(db, query).await
}
