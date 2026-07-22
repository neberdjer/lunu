use chrono::{DateTime, Utc};
use lunu_core::models::{Activity, BlocklistEntry, Download, QualityProfile, Request};
use serde::Serialize;

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct BlocklistResponse {
	pub id: String,
	pub request_id: String,
	pub download_url: String,
	pub created_at: DateTime<Utc>,
}

impl From<&BlocklistEntry> for BlocklistResponse {
	fn from(entry: &BlocklistEntry) -> Self {
		Self {
			id: entry.id.clone(),
			request_id: entry.request_id.clone(),
			download_url: entry.download_url.clone(),
			created_at: entry.created_at,
		}
	}
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct ActivityResponse {
	pub id: String,
	pub request_id: Option<String>,
	pub media_id: Option<String>,
	pub event: String,
	pub detail: Option<String>,
	pub actor: Option<String>,
	pub at: DateTime<Utc>,
}

impl From<&Activity> for ActivityResponse {
	fn from(activity: &Activity) -> Self {
		Self {
			id: activity.id.clone(),
			request_id: activity.request_id.clone(),
			media_id: activity.media_id.clone(),
			event: activity.event.clone(),
			detail: activity.detail.clone(),
			actor: activity.actor.clone(),
			at: activity.at,
		}
	}
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct RequestResponse {
	pub id: String,
	pub user_id: String,
	pub asin: Option<String>,
	pub title: String,
	pub author: Option<String>,
	pub cover_url: Option<String>,
	pub status: String,
	pub approved_by: Option<String>,
	pub notes: Option<String>,
	pub quality_profile_id: Option<String>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

impl From<&Request> for RequestResponse {
	fn from(request: &Request) -> Self {
		Self {
			id: request.id.clone(),
			user_id: request.user_id.clone(),
			asin: request.asin.clone(),
			title: request.title.clone(),
			author: request.author.clone(),
			cover_url: request.cover_url.clone(),
			status: request.status.to_string(),
			approved_by: request.approved_by.clone(),
			notes: request.notes.clone(),
			quality_profile_id: request.quality_profile_id.clone(),
			created_at: request.created_at,
			updated_at: request.updated_at,
		}
	}
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct DownloadResponse {
	pub id: String,
	pub request_id: String,
	pub client: String,
	pub category: String,
	pub release_title: String,
	pub indexer: String,
	pub state: String,
	pub progress: i64,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

impl From<&Download> for DownloadResponse {
	fn from(download: &Download) -> Self {
		Self {
			id: download.id.clone(),
			request_id: download.request_id.clone(),
			client: download.client.clone(),
			category: download.category.clone(),
			release_title: download.release_title.clone(),
			indexer: download.indexer.clone(),
			state: download.state.to_string(),
			progress: download.progress,
			created_at: download.created_at,
			updated_at: download.updated_at,
		}
	}
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct QualityProfileResponse {
	pub id: String,
	pub name: String,
	pub allowed_formats: Vec<String>,
	pub preferred_formats: Vec<String>,
	pub min_seeders: i64,
	pub min_size_mb: Option<i64>,
	pub max_size_mb: Option<i64>,
	pub seeder_weight: i64,
	pub format_weight: i64,
	pub preferred_keywords: Vec<String>,
	pub avoided_keywords: Vec<String>,
	pub keyword_weight: i64,
	pub preferred_protocol: Option<String>,
	pub protocol_weight: i64,
	pub is_default: bool,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

impl From<&QualityProfile> for QualityProfileResponse {
	fn from(profile: &QualityProfile) -> Self {
		Self {
			id: profile.id.clone(),
			name: profile.name.clone(),
			allowed_formats: profile.allowed_formats.clone(),
			preferred_formats: profile.preferred_formats.clone(),
			min_seeders: profile.min_seeders,
			min_size_mb: profile.min_size_mb,
			max_size_mb: profile.max_size_mb,
			seeder_weight: profile.seeder_weight,
			format_weight: profile.format_weight,
			preferred_keywords: profile.preferred_keywords.clone(),
			avoided_keywords: profile.avoided_keywords.clone(),
			keyword_weight: profile.keyword_weight,
			preferred_protocol: profile
				.preferred_protocol
				.map(|protocol| protocol.as_str().to_string()),
			protocol_weight: profile.protocol_weight,
			is_default: profile.is_default,
			created_at: profile.created_at,
			updated_at: profile.updated_at,
		}
	}
}
