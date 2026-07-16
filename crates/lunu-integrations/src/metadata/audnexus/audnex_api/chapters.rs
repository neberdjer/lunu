use lunu_core::models::{Chapter, Chapters};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AudnexusChapters {
	asin: String,
	runtime_length_ms: Option<i64>,
	is_accurate: Option<bool>,
	brand_intro_duration_ms: Option<i64>,
	brand_outro_duration_ms: Option<i64>,
	#[serde(default)]
	chapters: Vec<AudnexusChapter>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AudnexusChapter {
	title: String,
	start_offset_ms: i64,
	length_ms: i64,
}

impl AudnexusChapters {
	pub(super) fn into_chapters(self) -> Chapters {
		Chapters {
			asin: self.asin,
			runtime_ms: self.runtime_length_ms,
			chapters: self
				.chapters
				.into_iter()
				.map(|chapter| Chapter {
					title: chapter.title,
					start_offset_ms: chapter.start_offset_ms,
					length_ms: chapter.length_ms,
				})
				.collect(),
			is_accurate: self.is_accurate,
			brand_intro_duration_ms: self.brand_intro_duration_ms,
			brand_outro_duration_ms: self.brand_outro_duration_ms,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const AUDNEXUS_CHAPTERS: &str = r#"{
		"asin": "1705009050",
		"runtimeLengthMs": 39724444,
		"isAccurate": true,
		"brandIntroDurationMs": 2043,
		"brandOutroDurationMs": 5061,
		"chapters": [
			{"title": "Opening Credits", "startOffsetMs": 0, "lengthMs": 12000},
			{"title": "Chapter 1", "startOffsetMs": 12000, "lengthMs": 1200000}
		]
	}"#;

	#[test]
	fn parses_audnexus_chapters() {
		let chapters = serde_json::from_str::<AudnexusChapters>(AUDNEXUS_CHAPTERS)
			.unwrap()
			.into_chapters();
		assert_eq!(chapters.runtime_ms, Some(39724444));
		assert_eq!(chapters.chapters.len(), 2);
		assert_eq!(chapters.chapters[1].start_offset_ms, 12000);
		assert_eq!(chapters.chapters[1].title, "Chapter 1");
		assert_eq!(chapters.is_accurate, Some(true));
		assert_eq!(chapters.brand_intro_duration_ms, Some(2043));
		assert_eq!(chapters.brand_outro_duration_ms, Some(5061));
	}

	#[test]
	fn a_response_without_brand_offsets_still_parses() {
		let chapters = serde_json::from_str::<AudnexusChapters>(
			r#"{"asin": "X", "chapters": [], "runtimeLengthMs": 1}"#,
		)
		.unwrap()
		.into_chapters();
		assert_eq!(chapters.is_accurate, None);
		assert_eq!(chapters.brand_intro_duration_ms, None);
	}
}
