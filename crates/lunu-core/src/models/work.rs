use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Work {
	pub id: String,
	pub title: String,
	pub author: Option<String>,
	pub cover_url: Option<String>,
	pub created_at: DateTime<Utc>,
}

pub fn normalize(value: &str) -> String {
	value
		.split_whitespace()
		.collect::<Vec<_>>()
		.join(" ")
		.to_lowercase()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn normalization_is_the_same_answer_on_every_backend() {
		assert_eq!(normalize("  The   Hobbit "), "the hobbit");
		assert_eq!(normalize("THE HOBBIT"), normalize("the hobbit"));
	}

	#[test]
	fn normalization_is_unicode_aware_rather_than_ascii_only() {
		assert_eq!(
			normalize("LES MIS\u{c9}RABLES"),
			"les mis\u{e9}rables",
			"the bundled sqlite lowercases ascii only, so this policy cannot live in sql"
		);
	}
}
