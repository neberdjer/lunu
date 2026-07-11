mod api_key;
mod invite;
mod metadata_cache;
mod quality_profile;
mod request;
mod session;
mod settings;
mod user;
mod user_settings;

pub use api_key::SqlxApiKeyRepo;
pub use invite::SqlxInviteRepo;
pub use metadata_cache::SqlxMetadataCacheRepo;
pub use quality_profile::SqlxQualityProfileRepo;
pub use request::SqlxRequestRepo;
pub use session::SqlxSessionRepo;
pub use settings::SqlxSettingsRepo;
pub use user::SqlxUserRepo;
pub use user_settings::SqlxUserSettingsRepo;

use lunu_core::Result;
use sqlx::any::AnyRow;

pub(crate) fn map_row_opt<T>(
	row: Option<AnyRow>,
	map: fn(&AnyRow) -> Result<T>,
) -> Result<Option<T>> {
	row.as_ref().map(map).transpose()
}

pub(crate) fn map_rows<T>(rows: Vec<AnyRow>, map: fn(&AnyRow) -> Result<T>) -> Result<Vec<T>> {
	rows.iter().map(map).collect()
}
