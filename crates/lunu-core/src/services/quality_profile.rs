use std::sync::Arc;

use chrono::Utc;

use crate::consts::reasons;
use crate::models::QualityProfile;
use crate::repo::QualityProfileRepo;
use crate::services::new_id;
use crate::{Error, Result};

pub struct QualityProfileInput {
	pub name: String,
	pub allowed_formats: Vec<String>,
	pub preferred_formats: Vec<String>,
	pub min_seeders: i64,
	pub min_size_mb: Option<i64>,
	pub max_size_mb: Option<i64>,
	pub seeder_weight: i64,
	pub format_weight: i64,
	pub is_default: bool,
}

pub struct QualityProfileService {
	repo: Arc<dyn QualityProfileRepo>,
}

impl QualityProfileService {
	pub fn new(repo: Arc<dyn QualityProfileRepo>) -> Self {
		Self { repo }
	}

	pub async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<QualityProfile>> {
		self.repo.list_page(limit, offset).await
	}

	pub async fn count(&self) -> Result<i64> {
		self.repo.count().await
	}

	pub async fn get(&self, id: &str) -> Result<Option<QualityProfile>> {
		self.repo.find_by_id(id).await
	}

	pub async fn require(&self, id: &str) -> Result<QualityProfile> {
		self.repo
			.find_by_id(id)
			.await?
			.ok_or_else(|| Error::Validation(reasons::UNKNOWN_PROFILE.to_string()))
	}

	pub async fn create(&self, input: QualityProfileInput) -> Result<QualityProfile> {
		let name = validate_name(&input.name)?;
		let now = Utc::now();
		let profile = QualityProfile {
			id: new_id(),
			name,
			allowed_formats: input.allowed_formats,
			preferred_formats: input.preferred_formats,
			min_seeders: input.min_seeders,
			min_size_mb: input.min_size_mb,
			max_size_mb: input.max_size_mb,
			seeder_weight: input.seeder_weight,
			format_weight: input.format_weight,
			is_default: input.is_default,
			created_at: now,
			updated_at: now,
		};

		self.repo.create(&profile).await?;
		if profile.is_default {
			self.repo.set_default(&profile.id).await?;
		}
		Ok(profile)
	}

	pub async fn update(&self, id: &str, input: QualityProfileInput) -> Result<QualityProfile> {
		let name = validate_name(&input.name)?;
		let mut profile = self
			.repo
			.find_by_id(id)
			.await?
			.ok_or_else(|| Error::NotFound(format!("quality profile {id}")))?;

		profile.name = name;
		profile.allowed_formats = input.allowed_formats;
		profile.preferred_formats = input.preferred_formats;
		profile.min_seeders = input.min_seeders;
		profile.min_size_mb = input.min_size_mb;
		profile.max_size_mb = input.max_size_mb;
		profile.seeder_weight = input.seeder_weight;
		profile.format_weight = input.format_weight;
		profile.is_default = input.is_default;
		profile.updated_at = Utc::now();

		self.repo.update(&profile).await?;
		if profile.is_default {
			self.repo.set_default(&profile.id).await?;
		}
		Ok(profile)
	}

	pub async fn delete(&self, id: &str) -> Result<()> {
		self.repo.delete(id).await
	}
}

fn validate_name(name: &str) -> Result<String> {
	let trimmed = name.trim();
	if trimmed.is_empty() {
		return Err(Error::Validation(
			reasons::PROFILE_NAME_REQUIRED.to_string(),
		));
	}
	Ok(trimmed.to_string())
}
