use std::path::PathBuf;

use crate::consts::library::UNKNOWN_AUTHOR;
use crate::consts::merge::MERGE_OUTPUT_EXTENSION;

const PLACEHOLDER: &str = "unknown";

pub fn destination(
	library: &str,
	author: Option<&str>,
	title: &str,
	series: Option<&str>,
	sequence: Option<&str>,
) -> String {
	let mut path = PathBuf::from(library);
	path.push(sanitize(author.unwrap_or(UNKNOWN_AUTHOR)));
	match series {
		Some(series) => {
			path.push(sanitize(series));
			path.push(sanitize(&book_folder(title, sequence)));
		}
		None => path.push(sanitize(title)),
	}
	path.to_string_lossy().into_owned()
}

fn book_folder(title: &str, sequence: Option<&str>) -> String {
	match sequence.map(str::trim).filter(|value| !value.is_empty()) {
		Some(sequence) => format!("Vol {sequence} - {title}"),
		None => title.to_string(),
	}
}

pub fn merged_file(directory: &str, title: &str) -> String {
	let mut path = PathBuf::from(directory);
	path.push(format!("{}.{MERGE_OUTPUT_EXTENSION}", sanitize(title)));
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
	fn the_merged_file_lands_beside_its_sources_with_a_safe_name() {
		assert_eq!(
			merged_file("/library/Tolkien/The Hobbit", "The Hobbit"),
			"/library/Tolkien/The Hobbit/The Hobbit.m4b"
		);
		assert_eq!(
			merged_file("/library/A/B", "Under: The/Sea"),
			"/library/A/B/Under_ The_Sea.m4b",
			"a title cannot escape its own directory through the merged file name"
		);
	}

	#[test]
	fn a_series_book_lands_where_audiobookshelf_parses_its_volume() {
		assert_eq!(
			destination(
				"/library",
				Some("Isaac Asimov"),
				"Foundation",
				Some("Foundation"),
				Some("1")
			),
			"/library/Isaac Asimov/Foundation/Vol 1 - Foundation",
			"audiobookshelf reads the series from the middle folder and the volume from the leaf"
		);
		assert_eq!(
			destination(
				"/library",
				Some("Isaac Asimov"),
				"Foundation",
				Some("Foundation"),
				None
			),
			"/library/Isaac Asimov/Foundation/Foundation",
			"a series with no position still nests, but invents no volume number"
		);
	}

	#[test]
	fn builds_author_title_path() {
		assert_eq!(
			destination("/library", Some("Tolkien"), "The Hobbit", None, None),
			"/library/Tolkien/The Hobbit"
		);
	}

	#[test]
	fn defaults_missing_author() {
		assert_eq!(
			destination("/library", None, "Loose Book", None, None),
			"/library/Unknown Author/Loose Book"
		);
	}

	#[test]
	fn sanitizes_path_separators_and_control_chars() {
		assert_eq!(
			destination("/library", Some("A/B:C"), "Title\t?<>", None, None),
			"/library/A_B_C/Title____"
		);
	}

	#[test]
	fn falls_back_when_component_empties_out() {
		assert_eq!(
			destination("/library", Some("..."), "///", None, None),
			"/library/unknown/___"
		);
	}
}
