use serde::{Deserialize, Serialize};

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
