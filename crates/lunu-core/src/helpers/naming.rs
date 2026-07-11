use std::path::PathBuf;

use crate::consts::library::UNKNOWN_AUTHOR;

const PLACEHOLDER: &str = "unknown";

pub fn destination(library: &str, author: Option<&str>, title: &str) -> String {
	let mut path = PathBuf::from(library);
	path.push(sanitize(author.unwrap_or(UNKNOWN_AUTHOR)));
	path.push(sanitize(title));
	path.to_string_lossy().into_owned()
}

fn sanitize(component: &str) -> String {
	let cleaned: String = component
		.chars()
		.map(|c| if is_unsafe(c) { '_' } else { c })
		.collect();
	let trimmed = cleaned.trim_matches(|c: char| c.is_whitespace() || c == '.');
	if trimmed.is_empty() {
		PLACEHOLDER.to_string()
	} else {
		trimmed.to_string()
	}
}

fn is_unsafe(c: char) -> bool {
	c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn builds_author_title_path() {
		assert_eq!(
			destination("/library", Some("Tolkien"), "The Hobbit"),
			"/library/Tolkien/The Hobbit"
		);
	}

	#[test]
	fn defaults_missing_author() {
		assert_eq!(
			destination("/library", None, "Loose Book"),
			"/library/Unknown Author/Loose Book"
		);
	}

	#[test]
	fn sanitizes_path_separators_and_control_chars() {
		assert_eq!(
			destination("/library", Some("A/B:C"), "Title\t?<>"),
			"/library/A_B_C/Title____"
		);
	}

	#[test]
	fn falls_back_when_component_empties_out() {
		assert_eq!(
			destination("/library", Some("..."), "///"),
			"/library/unknown/___"
		);
	}
}
