use super::MetadataService;
use crate::consts::metadata::{
	DEFAULT_METADATA_REGION, METADATA_REGION_SETTING, VALID_METADATA_REGIONS,
};
use crate::consts::reasons;
use crate::models::ExternalId;
use crate::{Error, Result};

impl MetadataService {
	pub async fn current_region(&self) -> Result<String> {
		self.region().await
	}

	pub async fn region_or_current(&self, region: Option<String>) -> Result<String> {
		match region {
			Some(region) => Ok(region),
			None => self.region().await,
		}
	}

	pub(super) async fn region_for(&self, id: &ExternalId) -> Result<String> {
		match id.region.as_deref() {
			Some(region) => Self::validate_region(region),
			None => self.region().await,
		}
	}

	pub(super) async fn region(&self) -> Result<String> {
		let region = self
			.settings
			.get(METADATA_REGION_SETTING)
			.await?
			.filter(|value| !value.trim().is_empty())
			.unwrap_or_else(|| DEFAULT_METADATA_REGION.to_string());
		Self::validate_region(&region)
	}

	fn validate_region(region: &str) -> Result<String> {
		let region = region.trim().to_ascii_lowercase();
		if !VALID_METADATA_REGIONS.contains(&region.as_str()) {
			return Err(Error::Validation(reasons::INVALID_REGION.to_string()));
		}
		Ok(region)
	}
}
