use crate::consts::scoring::{
	FREELEECH_TOKENS, LANGUAGE_TOKENS, MAX_PLAUSIBLE_KBPS, MIN_PLAUSIBLE_KBPS,
};

const BITRATE_SUFFIXES: &[&str] = &["k", "kbps", "kbit"];

pub fn tokenize(value: &str) -> String {
	value
		.to_lowercase()
		.split(|c: char| !c.is_alphanumeric())
		.filter(|token| !token.is_empty())
		.collect::<Vec<_>>()
		.join(" ")
}

pub fn contains_token(padded: &str, token: &str) -> bool {
	if token.is_empty() {
		return false;
	}
	match token.split_once(' ') {
		Some(_) => padded.contains(&format!(" {token} ")),
		None => padded.split(' ').any(|word| word == token),
	}
}

pub fn detect_bitrate(padded: &str) -> Option<i64> {
	padded
		.split(' ')
		.filter_map(|token| {
			let digits = token.trim_end_matches(|c: char| c.is_ascii_alphabetic());
			if digits.is_empty() || !BITRATE_SUFFIXES.contains(&&token[digits.len()..]) {
				return None;
			}
			digits.parse::<i64>().ok()
		})
		.filter(|value| (MIN_PLAUSIBLE_KBPS..=MAX_PLAUSIBLE_KBPS).contains(value))
		.max()
}

pub fn detect_language(padded: &str) -> Option<&'static str> {
	LANGUAGE_TOKENS
		.iter()
		.find(|(_, tokens)| tokens.iter().any(|token| contains_token(padded, token)))
		.map(|(code, _)| *code)
}

pub fn is_freeleech(padded: &str) -> bool {
	FREELEECH_TOKENS
		.iter()
		.any(|token| contains_token(padded, token))
}

#[cfg(test)]
mod tests {
	use super::*;

	fn padded(title: &str) -> String {
		format!(" {} ", tokenize(title))
	}

	#[test]
	fn a_bitrate_is_read_from_the_shapes_trackers_actually_use() {
		assert_eq!(detect_bitrate(&padded("The Hobbit [M4B 64kbps]")), Some(64));
		assert_eq!(detect_bitrate(&padded("The Hobbit 128 kbps")), None);
		assert_eq!(detect_bitrate(&padded("The Hobbit 128kbit/s")), Some(128));
		assert_eq!(detect_bitrate(&padded("The Hobbit {32k}")), Some(32));
	}

	#[test]
	fn a_title_without_a_bitrate_reports_none_rather_than_guessing() {
		assert_eq!(detect_bitrate(&padded("The Hobbit [M4B]")), None);
		assert_eq!(detect_bitrate(&padded("The Hobbit")), None);
	}

	#[test]
	fn a_file_size_is_never_mistaken_for_a_bitrate() {
		assert_eq!(
			detect_bitrate(&padded("The Hobbit 700kb")),
			None,
			"kb is a size suffix, and 700 is outside any plausible audiobook bitrate"
		);
		assert_eq!(
			detect_bitrate(&padded("The Hobbit 1.5k")),
			None,
			"a decimal reads as a size, not a bitrate"
		);
	}

	#[test]
	fn the_highest_bitrate_wins_when_a_title_lists_several() {
		assert_eq!(
			detect_bitrate(&padded("Collection 32k to 128kbps")),
			Some(128)
		);
	}

	#[test]
	fn languages_are_detected_from_word_and_code_forms() {
		assert_eq!(detect_language(&padded("The Hobbit [German]")), Some("de"));
		assert_eq!(detect_language(&padded("Der Hobbit GER m4b")), Some("de"));
		assert_eq!(detect_language(&padded("The Hobbit English")), Some("en"));
		assert_eq!(detect_language(&padded("The Hobbit")), None);
	}

	#[test]
	fn a_language_token_must_be_a_whole_word() {
		assert_eq!(
			detect_language(&padded("Germany Calling")),
			None,
			"'germany' is not 'german', and substring matching would misfile the release"
		);
		assert_eq!(detect_language(&padded("The Engineer")), None);
	}

	#[test]
	fn freeleech_is_recognised_in_its_common_spellings() {
		assert!(is_freeleech(&padded("The Hobbit [FreeLeech]")));
		assert!(is_freeleech(&padded("The Hobbit (free leech)")));
		assert!(is_freeleech(&padded("The Hobbit [FL]")));
		assert!(!is_freeleech(&padded("The Hobbit")));
		assert!(
			!is_freeleech(&padded("The Flute Player")),
			"'fl' must be a token, not a prefix"
		);
	}
}
