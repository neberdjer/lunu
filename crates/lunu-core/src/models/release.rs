use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::consts::download::COMPLETE_PROGRESS;
use crate::consts::reasons;
use crate::{Error, Result};

pub const BYTES_PER_MB: i64 = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
	Torrent,
	Usenet,
}

impl Protocol {
	pub fn as_str(&self) -> &'static str {
		match self {
			Protocol::Torrent => "torrent",
			Protocol::Usenet => "usenet",
		}
	}

	pub fn has_swarm(&self) -> bool {
		matches!(self, Protocol::Torrent)
	}

	pub fn owes_seeding_at(&self, progress: i64) -> bool {
		self.has_swarm() && progress >= COMPLETE_PROGRESS
	}
}

impl FromStr for Protocol {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		match value {
			"torrent" => Ok(Protocol::Torrent),
			"usenet" => Ok(Protocol::Usenet),
			_ => Err(Error::Validation(reasons::PROTOCOL_UNKNOWN.to_string())),
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
	pub title: String,
	pub indexer: String,
	pub protocol: Protocol,
	pub size: i64,
	pub seeders: i64,
	pub leechers: i64,
	pub download_url: String,
	pub info_hash: Option<String>,
	pub info_url: Option<String>,
	pub publish_date: Option<String>,
}

impl Release {
	pub fn size_mb(&self) -> i64 {
		self.size / BYTES_PER_MB
	}
}

#[derive(Debug, Clone, Serialize)]
pub struct ScoredRelease {
	pub release: Release,
	pub score: i64,
}
