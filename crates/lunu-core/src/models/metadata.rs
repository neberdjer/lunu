use chrono::{DateTime, Utc};

use super::identity::{ExternalId, IdScheme};
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
	pub ids: Vec<ExternalId>,
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
	#[serde(default)]
	pub tags: Vec<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub format_type: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub rating: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub is_adult: Option<bool>,
}

impl Book {
	pub fn id(&self, scheme: IdScheme) -> Option<&str> {
		self.ids.iter().find_map(|id| id.value_for(scheme))
	}

	pub fn primary_id(&self) -> Option<&ExternalId> {
		self.ids.first()
	}

	pub fn asin(&self) -> Option<&str> {
		self.id(IdScheme::Asin)
	}

	pub fn isbn(&self) -> Option<&str> {
		self.id(IdScheme::Isbn)
	}
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
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub is_accurate: Option<bool>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub brand_intro_duration_ms: Option<i64>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub brand_outro_duration_ms: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct MetadataCacheEntry {
	pub provider: String,
	pub kind: String,
	pub key: String,
	pub payload: String,
	pub fetched_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn the_primary_id_is_the_sources_native_handle() {
		let mut book = Book {
			ids: vec![ExternalId::asin("B123"), ExternalId::isbn("9780007487295")],
			title: String::new(),
			subtitle: None,
			authors: Vec::new(),
			author_asins: Vec::new(),
			narrators: Vec::new(),
			series: Vec::new(),
			description: None,
			cover_url: None,
			release_date: None,
			runtime_minutes: None,
			language: None,
			publisher: None,
			genres: Vec::new(),
			tags: Vec::new(),
			format_type: None,
			rating: None,
			is_adult: None,
		};
		assert_eq!(
			book.primary_id(),
			Some(&ExternalId::asin("B123")),
			"a provider must list the id it natively speaks first, or the wire id it hands \
			 out routes its own results through a different source"
		);
		book.ids.clear();
		assert_eq!(book.primary_id(), None);
	}
}
