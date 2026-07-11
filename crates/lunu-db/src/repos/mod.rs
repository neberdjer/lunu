mod activity;
mod api_key;
mod download;
mod invite;
mod job;
mod metadata_cache;
mod quality_profile;
mod request;
mod session;
mod settings;
mod user;
mod user_settings;

pub use activity::SqlxActivityRepo;
pub use api_key::SqlxApiKeyRepo;
pub use download::SqlxDownloadRepo;
pub use invite::SqlxInviteRepo;
pub use job::SqlxJobRepo;
pub use metadata_cache::SqlxMetadataCacheRepo;
pub use quality_profile::SqlxQualityProfileRepo;
pub use request::SqlxRequestRepo;
pub use session::SqlxSessionRepo;
pub use settings::SqlxSettingsRepo;
pub use user::SqlxUserRepo;
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
