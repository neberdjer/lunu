use std::cmp::Ordering;
use std::fs;
use std::iter::Peekable;
use std::path::{Path, PathBuf};

use lunu_core::Result;
use lunu_core::helpers::format::is_audio_extension;

use crate::integration_error;

pub(super) fn collect(dir: &Path, exclude: &[PathBuf]) -> Result<Vec<PathBuf>> {
	let mut found = Vec::new();
	walk(
		dir,
		&mut |path: &Path| !exclude.iter().any(|skip| skip == path) && is_mergeable(path),
		&mut found,
	)?;
	found.sort_by(|a, b| natural_cmp(&a.to_string_lossy(), &b.to_string_lossy()));
	Ok(found)
}

pub(super) fn walk_all(dir: &Path, found: &mut Vec<PathBuf>) -> Result<()> {
	walk(dir, &mut |_| true, found)
}

fn walk(dir: &Path, keep: &mut impl FnMut(&Path) -> bool, found: &mut Vec<PathBuf>) -> Result<()> {
	for entry in fs::read_dir(dir).map_err(integration_error)? {
		let entry = entry.map_err(integration_error)?;
		let path = entry.path();
		if entry.file_type().map_err(integration_error)?.is_dir() {
			walk(&path, keep, found)?;
		} else if keep(&path) {
			found.push(path);
		}
	}
	Ok(())
}

fn is_mergeable(path: &Path) -> bool {
	path.extension()
		.and_then(|extension| extension.to_str())
		.is_some_and(is_audio_extension)
}

pub(super) fn chapter_title(path: &Path) -> String {
	path.file_stem()
		.map(|stem| stem.to_string_lossy().trim().to_string())
		.filter(|stem| !stem.is_empty())
		.unwrap_or_else(|| "Chapter".to_string())
}

fn natural_cmp(a: &str, b: &str) -> Ordering {
	let mut left = a.chars().peekable();
	let mut right = b.chars().peekable();

	loop {
		match (left.peek().copied(), right.peek().copied()) {
			(None, None) => return Ordering::Equal,
			(None, Some(_)) => return Ordering::Less,
			(Some(_), None) => return Ordering::Greater,
			(Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
				let ordering = take_number(&mut left).cmp(&take_number(&mut right));
				if ordering != Ordering::Equal {
					return ordering;
				}
			}
			(Some(x), Some(y)) => {
				let ordering = x.to_ascii_lowercase().cmp(&y.to_ascii_lowercase());
				if ordering != Ordering::Equal {
					return ordering;
				}
				left.next();
				right.next();
			}
		}
	}
}

fn take_number(chars: &mut Peekable<impl Iterator<Item = char>>) -> u64 {
	let mut value: u64 = 0;
	while let Some(digit) = chars.peek().and_then(|c| c.to_digit(10)) {
		value = value.saturating_mul(10).saturating_add(digit as u64);
		chars.next();
	}
	value
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn padded_and_prefixed_names_still_sort_in_order() {
		let mut names = vec![
			"Part 2 - Track 09.mp3",
			"Part 1 - Track 10.mp3",
			"Part 1 - Track 9.mp3",
			"Part 1 - Track 2.mp3",
		];
		names.sort_by(|a, b| natural_cmp(a, b));
		assert_eq!(
			names,
			vec![
				"Part 1 - Track 2.mp3",
				"Part 1 - Track 9.mp3",
				"Part 1 - Track 10.mp3",
				"Part 2 - Track 09.mp3",
			],
			"lexicographic order would put 10 before 2, and padding must not change the answer"
		);
	}

	#[test]
	fn only_audio_extensions_are_mergeable() {
		assert!(is_mergeable(Path::new("/x/01.MP3")));
		assert!(is_mergeable(Path::new("/x/01.m4a")));
		assert!(!is_mergeable(Path::new("/x/cover.jpg")));
		assert!(!is_mergeable(Path::new("/x/notes.txt")));
		assert!(!is_mergeable(Path::new("/x/noextension")));
	}

	#[test]
	fn a_chapter_takes_its_name_from_the_file_stem() {
		assert_eq!(
			chapter_title(Path::new("/x/01 - The Shire.mp3")),
			"01 - The Shire"
		);
	}
}
