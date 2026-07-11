use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::models::QualityProfile;
use lunu_core::repo::QualityProfileRepo;
use sqlx::Row;
use sqlx::any::AnyRow;

use super::{map_row_opt, map_rows};
use crate::convert::{bool_to_int, format_dt, int_to_bool, join_list, parse_dt, split_list};
use crate::{Db, db_error};

pub struct SqlxQualityProfileRepo {
	db: Db,
}

impl SqlxQualityProfileRepo {
	pub fn new(db: Db) -> Self {
		Self { db }
	}
}

fn map_profile(row: &AnyRow) -> Result<QualityProfile> {
	let allowed_formats: String = row.try_get("allowed_formats").map_err(db_error)?;
	let preferred_formats: String = row.try_get("preferred_formats").map_err(db_error)?;
	let is_default: i64 = row.try_get("is_default").map_err(db_error)?;
	let created_at: String = row.try_get("created_at").map_err(db_error)?;
	let updated_at: String = row.try_get("updated_at").map_err(db_error)?;

	Ok(QualityProfile {
		id: row.try_get("id").map_err(db_error)?,
		name: row.try_get("name").map_err(db_error)?,
		allowed_formats: split_list(&allowed_formats),
		preferred_formats: split_list(&preferred_formats),
		min_seeders: row.try_get("min_seeders").map_err(db_error)?,
		min_size_mb: row.try_get("min_size_mb").map_err(db_error)?,
		max_size_mb: row.try_get("max_size_mb").map_err(db_error)?,
		seeder_weight: row.try_get("seeder_weight").map_err(db_error)?,
		format_weight: row.try_get("format_weight").map_err(db_error)?,
		is_default: int_to_bool(is_default),
		created_at: parse_dt(&created_at)?,
		updated_at: parse_dt(&updated_at)?,
	})
}

#[async_trait]
impl QualityProfileRepo for SqlxQualityProfileRepo {
	async fn create(&self, profile: &QualityProfile) -> Result<()> {
		sqlx::query(
			"INSERT INTO quality_profiles \
			 (id, name, allowed_formats, preferred_formats, min_seeders, min_size_mb, max_size_mb, \
			 seeder_weight, format_weight, is_default, created_at, updated_at) \
			 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
		)
		.bind(&profile.id)
		.bind(&profile.name)
		.bind(join_list(&profile.allowed_formats))
		.bind(join_list(&profile.preferred_formats))
		.bind(profile.min_seeders)
		.bind(profile.min_size_mb)
		.bind(profile.max_size_mb)
		.bind(profile.seeder_weight)
		.bind(profile.format_weight)
		.bind(bool_to_int(profile.is_default))
		.bind(format_dt(profile.created_at))
		.bind(format_dt(profile.updated_at))
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(())
	}

	async fn update(&self, profile: &QualityProfile) -> Result<()> {
		sqlx::query(
			"UPDATE quality_profiles SET \
			 name = $1, allowed_formats = $2, preferred_formats = $3, min_seeders = $4, \
			 min_size_mb = $5, max_size_mb = $6, seeder_weight = $7, format_weight = $8, \
			 is_default = $9, updated_at = $10 WHERE id = $11",
		)
		.bind(&profile.name)
		.bind(join_list(&profile.allowed_formats))
		.bind(join_list(&profile.preferred_formats))
		.bind(profile.min_seeders)
		.bind(profile.min_size_mb)
		.bind(profile.max_size_mb)
		.bind(profile.seeder_weight)
		.bind(profile.format_weight)
		.bind(bool_to_int(profile.is_default))
		.bind(format_dt(profile.updated_at))
		.bind(&profile.id)
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(())
	}

	async fn find_by_id(&self, id: &str) -> Result<Option<QualityProfile>> {
		let row = sqlx::query("SELECT * FROM quality_profiles WHERE id = $1")
			.bind(id)
			.fetch_optional(&self.db)
			.await
			.map_err(db_error)?;
		map_row_opt(row, map_profile)
	}

	async fn find_default(&self) -> Result<Option<QualityProfile>> {
		let row = sqlx::query("SELECT * FROM quality_profiles WHERE is_default = 1 LIMIT 1")
			.fetch_optional(&self.db)
			.await
			.map_err(db_error)?;
		map_row_opt(row, map_profile)
	}

	async fn list(&self) -> Result<Vec<QualityProfile>> {
		let rows = sqlx::query("SELECT * FROM quality_profiles ORDER BY name")
			.fetch_all(&self.db)
			.await
			.map_err(db_error)?;
		map_rows(rows, map_profile)
	}

	async fn set_default(&self, id: &str) -> Result<()> {
		sqlx::query("UPDATE quality_profiles SET is_default = CASE WHEN id = $1 THEN 1 ELSE 0 END")
			.bind(id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}

	async fn delete(&self, id: &str) -> Result<()> {
		sqlx::query("DELETE FROM quality_profiles WHERE id = $1")
			.bind(id)
			.execute(&self.db)
			.await
			.map_err(db_error)?;
		Ok(())
	}
}
