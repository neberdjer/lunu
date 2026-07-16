use chrono::{DateTime, Utc};

use crate::consts::scoring::{
	DEFAULT_AVOIDED_KEYWORDS, DEFAULT_FORMAT_WEIGHT, DEFAULT_KEYWORD_WEIGHT, DEFAULT_MIN_SEEDERS,
	DEFAULT_PREFERRED_FORMATS, DEFAULT_SEEDER_WEIGHT,
};

#[derive(Debug, Clone)]
pub struct QualityProfile {
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
	pub is_default: bool,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

impl QualityProfile {
	pub fn builtin_default() -> Self {
		let now = Utc::now();
		Self {
			id: "builtin-default".to_string(),
			name: "Default".to_string(),
			allowed_formats: Vec::new(),
			preferred_formats: DEFAULT_PREFERRED_FORMATS
				.iter()
				.map(|format| (*format).to_string())
				.collect(),
			min_seeders: DEFAULT_MIN_SEEDERS,
			min_size_mb: None,
			max_size_mb: None,
			seeder_weight: DEFAULT_SEEDER_WEIGHT,
			format_weight: DEFAULT_FORMAT_WEIGHT,
			preferred_keywords: Vec::new(),
			avoided_keywords: DEFAULT_AVOIDED_KEYWORDS
				.iter()
				.map(|keyword| (*keyword).to_string())
				.collect(),
			keyword_weight: DEFAULT_KEYWORD_WEIGHT,
			is_default: true,
			created_at: now,
			updated_at: now,
		}
	}
}
