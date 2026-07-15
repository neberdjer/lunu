use lunu_core::models::{Release, ScoredRelease};
use serde::Serialize;

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct ReleaseResponse {
	pub title: String,
	pub indexer: String,
	pub protocol: String,
	pub size: i64,
	pub seeders: i64,
	pub leechers: i64,
	pub download_url: String,
	pub info_hash: Option<String>,
	pub info_url: Option<String>,
	pub publish_date: Option<String>,
}

impl From<&Release> for ReleaseResponse {
	fn from(release: &Release) -> Self {
		Self {
			title: release.title.clone(),
			indexer: release.indexer.clone(),
			protocol: release.protocol.as_str().to_string(),
			size: release.size,
			seeders: release.seeders,
			leechers: release.leechers,
			download_url: release.download_url.clone(),
			info_hash: release.info_hash.clone(),
			info_url: release.info_url.clone(),
			publish_date: release.publish_date.clone(),
		}
	}
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct ScoredReleaseResponse {
	pub release: ReleaseResponse,
	pub score: i64,
}

impl From<&ScoredRelease> for ScoredReleaseResponse {
	fn from(scored: &ScoredRelease) -> Self {
		Self {
			release: ReleaseResponse::from(&scored.release),
			score: scored.score,
		}
	}
}
