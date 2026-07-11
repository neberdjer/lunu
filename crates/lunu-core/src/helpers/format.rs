use crate::consts::scoring::KNOWN_AUDIO_FORMATS;

pub fn detect_format(title: &str) -> Option<&'static str> {
	KNOWN_AUDIO_FORMATS
		.iter()
		.find(|&&format| {
			title
				.split(|c: char| !c.is_alphanumeric())
				.any(|token| token.eq_ignore_ascii_case(format))
		})
		.copied()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn detects_bracketed_format() {
		assert_eq!(detect_format("Author - Title (2020) [M4B]"), Some("m4b"));
	}

	#[test]
	fn detects_spaced_format() {
		assert_eq!(detect_format("Title 2020 MP3 64kbps"), Some("mp3"));
	}

	#[test]
	fn returns_none_when_absent() {
		assert_eq!(detect_format("Title with no format tag"), None);
	}

	#[test]
	fn prefers_higher_priority_format() {
		assert_eq!(
			detect_format("Title [M4B] also has mp3 fallback"),
			Some("m4b")
		);
	}
}
