use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::models::{Protocol, QualityProfile};
use lunu_core::repo::QualityProfileRepo;
use sqlx::Row;
use sqlx::any::AnyRow;

use super::{fetch_count, map_row_opt, map_rows};
use crate::convert::{
	bool_to_int, format_dt, int_to_bool, join_list, parse_dt, parse_enum, split_list,
};
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
	let preferred_keywords: String = row.try_get("preferred_keywords").map_err(db_error)?;
	let avoided_keywords: String = row.try_get("avoided_keywords").map_err(db_error)?;
	let is_default: i64 = row.try_get("is_default").map_err(db_error)?;
	let preferred_protocol: Option<String> = row.try_get("preferred_protocol").map_err(db_error)?;
	let allowed_languages: String = row.try_get("allowed_languages").map_err(db_error)?;
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
		preferred_keywords: split_list(&preferred_keywords),
		avoided_keywords: split_list(&avoided_keywords),
		keyword_weight: row.try_get("keyword_weight").map_err(db_error)?,
		preferred_protocol: preferred_protocol
			.as_deref()
			.map(parse_enum::<Protocol>)
			.transpose()?,
		protocol_weight: row.try_get("protocol_weight").map_err(db_error)?,
		min_bitrate_kbps: row.try_get("min_bitrate_kbps").map_err(db_error)?,
		bitrate_weight: row.try_get("bitrate_weight").map_err(db_error)?,
		allowed_languages: split_list(&allowed_languages),
		freeleech_weight: row.try_get("freeleech_weight").map_err(db_error)?,
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
			 seeder_weight, format_weight, preferred_keywords, avoided_keywords, keyword_weight, \
			 preferred_protocol, protocol_weight, min_bitrate_kbps, bitrate_weight, \
			 allowed_languages, freeleech_weight, is_default, created_at, updated_at) \
			 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, \
			 $18, $19, $20, $21)",
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
		.bind(join_list(&profile.preferred_keywords))
		.bind(join_list(&profile.avoided_keywords))
		.bind(profile.keyword_weight)
		.bind(profile.preferred_protocol.map(|protocol| protocol.as_str()))
		.bind(profile.protocol_weight)
		.bind(profile.min_bitrate_kbps)
		.bind(profile.bitrate_weight)
		.bind(join_list(&profile.allowed_languages))
		.bind(profile.freeleech_weight)
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
			 preferred_keywords = $9, avoided_keywords = $10, keyword_weight = $11, \
			 preferred_protocol = $12, protocol_weight = $13, min_bitrate_kbps = $14, \
			 bitrate_weight = $15, allowed_languages = $16, freeleech_weight = $17, \
			 is_default = $18, updated_at = $19 \
			 WHERE id = $20",
		)
		.bind(&profile.name)
		.bind(join_list(&profile.allowed_formats))
		.bind(join_list(&profile.preferred_formats))
		.bind(profile.min_seeders)
		.bind(profile.min_size_mb)
		.bind(profile.max_size_mb)
		.bind(profile.seeder_weight)
		.bind(profile.format_weight)
		.bind(join_list(&profile.preferred_keywords))
		.bind(join_list(&profile.avoided_keywords))
		.bind(profile.keyword_weight)
		.bind(profile.preferred_protocol.map(|protocol| protocol.as_str()))
		.bind(profile.protocol_weight)
		.bind(profile.min_bitrate_kbps)
		.bind(profile.bitrate_weight)
		.bind(join_list(&profile.allowed_languages))
		.bind(profile.freeleech_weight)
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

	async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<QualityProfile>> {
		let rows = sqlx::query("SELECT * FROM quality_profiles ORDER BY name LIMIT $1 OFFSET $2")
			.bind(limit)
			.bind(offset)
			.fetch_all(&self.db)
			.await
			.map_err(db_error)?;
		map_rows(rows, map_profile)
	}

	async fn count(&self) -> Result<i64> {
		fetch_count(
			&self.db,
			sqlx::query("SELECT COUNT(*) AS count FROM quality_profiles"),
		)
		.await
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
