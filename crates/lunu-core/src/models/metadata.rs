use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeriesRef {
	pub name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub position: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub asin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeriesSummary {
	pub name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub asin: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub author: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub cover_url: Option<String>,
	pub books_in_results: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Book {
	pub asin: String,
	pub title: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub subtitle: Option<String>,
	pub authors: Vec<String>,
	#[serde(default)]
	pub author_asins: Vec<String>,
	pub narrators: Vec<String>,
	pub series: Vec<SeriesRef>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub cover_url: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub release_date: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub runtime_minutes: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub language: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub publisher: Option<String>,
	pub genres: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
	pub title: String,
	pub start_offset_ms: i64,
	pub length_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapters {
	pub asin: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub runtime_ms: Option<i64>,
	pub chapters: Vec<Chapter>,
}

#[derive(Debug, Clone)]
pub struct MetadataCacheEntry {
	pub provider: String,
	pub kind: String,
	pub key: String,
	pub payload: String,
	pub fetched_at: DateTime<Utc>,
}
