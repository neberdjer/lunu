use std::str::FromStr;

use chrono::{DateTime, Utc};

use crate::consts::reasons;
use crate::{Error, Result};

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
	pub title: String,
	pub author: Option<String>,
	pub cover_url: Option<String>,
	pub series_name: Option<String>,
	pub series_sequence: Option<String>,
	pub library_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Media {
	pub id: String,
	pub asin: Option<String>,
	pub abs_item_id: Option<String>,
	pub title: String,
	pub author: Option<String>,
	pub cover_url: Option<String>,
	pub series_name: Option<String>,
	pub series_sequence: Option<String>,
	pub library_path: String,
	pub source: MediaSource,
	pub overridden: bool,
	pub request_id: Option<String>,
	pub created_at: DateTime<Utc>,
}
