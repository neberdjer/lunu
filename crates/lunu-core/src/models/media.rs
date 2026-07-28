use std::str::FromStr;

use chrono::{DateTime, Utc};

use crate::consts::reasons;
use crate::models::{ExternalId, Format};
use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MediaFilter {
	#[default]
	All,
	Unmatched,
	Mergeable,
}

impl FromStr for MediaFilter {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		match value {
			"all" => Ok(MediaFilter::All),
			"unmatched" => Ok(MediaFilter::Unmatched),
			"mergeable" => Ok(MediaFilter::Mergeable),
			_ => Err(Error::Validation(reasons::MEDIA_FILTER_UNKNOWN.to_string())),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MergeState {
	#[default]
	Idle,
	Queued,
	Merged,
	Skipped,
	Failed,
}

impl MergeState {
	pub fn is_merge_candidate(&self) -> bool {
		match self {
			MergeState::Idle | MergeState::Failed => true,
			MergeState::Queued | MergeState::Merged | MergeState::Skipped => false,
		}
	}

	pub const ALL: &'static [MergeState] = &[
		MergeState::Idle,
		MergeState::Queued,
		MergeState::Merged,
		MergeState::Skipped,
		MergeState::Failed,
	];

	pub fn as_str(&self) -> &'static str {
		match self {
			MergeState::Idle => "idle",
			MergeState::Queued => "queued",
			MergeState::Merged => "merged",
			MergeState::Skipped => "skipped",
			MergeState::Failed => "failed",
		}
	}
}

impl FromStr for MergeState {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		match value {
			"idle" => Ok(MergeState::Idle),
			"queued" => Ok(MergeState::Queued),
			"merged" => Ok(MergeState::Merged),
			"skipped" => Ok(MergeState::Skipped),
			"failed" => Ok(MergeState::Failed),
			_ => Err(Error::Validation(reasons::MERGE_STATE_UNKNOWN.to_string())),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaSource {
	Request,
	Abs,
}

impl MediaSource {
	pub fn as_str(&self) -> &'static str {
		match self {
			MediaSource::Request => "request",
			MediaSource::Abs => "abs",
		}
	}
}

impl FromStr for MediaSource {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		match value {
			"request" => Ok(MediaSource::Request),
			"abs" => Ok(MediaSource::Abs),
			_ => Err(Error::Validation(reasons::MEDIA_SOURCE_UNKNOWN.to_string())),
		}
	}
}

#[derive(Debug, Clone)]
pub struct LibraryItem {
	pub abs_item_id: String,
	pub asin: Option<String>,
	pub isbn: Option<String>,
	pub title: String,
	pub author: Option<String>,
	pub cover_url: Option<String>,
	pub series_name: Option<String>,
	pub series_sequence: Option<String>,
	pub library_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchedBy {
	Asin,
	Isbn,
	Title,
	Series,
	Fuzzy,
	Manual,
}

impl MatchedBy {
	pub fn as_str(&self) -> &'static str {
		match self {
			MatchedBy::Asin => "asin",
			MatchedBy::Isbn => "isbn",
			MatchedBy::Title => "title",
			MatchedBy::Series => "series",
			MatchedBy::Fuzzy => "fuzzy",
			MatchedBy::Manual => "manual",
		}
	}
}

impl FromStr for MatchedBy {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		match value {
			"asin" => Ok(MatchedBy::Asin),
			"isbn" => Ok(MatchedBy::Isbn),
			"title" => Ok(MatchedBy::Title),
			"series" => Ok(MatchedBy::Series),
			"fuzzy" => Ok(MatchedBy::Fuzzy),
			"manual" => Ok(MatchedBy::Manual),
			_ => Err(Error::Validation(reasons::MATCH_KIND_UNKNOWN.to_string())),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Media {
	pub id: String,
	pub work_id: Option<String>,
	pub format: Format,
	pub asin: Option<String>,
	pub abs_item_id: Option<String>,
	pub title: String,
	pub author: Option<String>,
	pub cover_url: Option<String>,
	pub series_name: Option<String>,
	pub series_sequence: Option<String>,
	pub library_path: String,
	pub merged_path: Option<String>,
	pub merge_state: MergeState,
	pub merge_detail: Option<String>,
	pub merge_backup_path: Option<String>,
	pub source: MediaSource,
	pub overridden: bool,
	pub matched_by: Option<MatchedBy>,
	pub metadata_region: Option<String>,
	pub request_id: Option<String>,
	pub created_at: DateTime<Utc>,
}

impl Media {
	pub fn external_id(&self) -> Option<ExternalId> {
		self.asin
			.as_deref()
			.map(|asin| ExternalId::asin_in_region(asin, self.metadata_region.clone()))
	}
}
