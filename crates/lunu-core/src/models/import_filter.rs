use std::path::Path;
use std::str::FromStr;

use crate::consts::library::{
	COVER_FILE, IMPORT_UNLISTED_EXTRAS, IMPORT_UNLISTED_KEEP, IMPORT_UNLISTED_SKIP,
	METADATA_OPF_FILE,
};
use crate::consts::reasons;
use crate::helpers::format::is_audio_extension;
use crate::{Error, Result};

const AUTHORED_FILES: &[&str] = &[METADATA_OPF_FILE, COVER_FILE];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Placement {
	#[default]
	Skip,
	Extras,
	Library,
}

impl FromStr for Placement {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self> {
		match value.trim() {
			IMPORT_UNLISTED_SKIP => Ok(Placement::Skip),
			IMPORT_UNLISTED_EXTRAS => Ok(Placement::Extras),
			IMPORT_UNLISTED_KEEP => Ok(Placement::Library),
			_ => Err(Error::Validation(
				reasons::IMPORT_UNLISTED_ACTION_UNKNOWN.to_string(),
			)),
		}
	}
}

#[derive(Debug, Clone, Default)]
pub struct ImportFilter {
	keep: Vec<String>,
	unlisted: Placement,
	authors_sidecar: bool,
}

impl ImportFilter {
	pub fn new(keep: &str, unlisted: Placement, authors_sidecar: bool) -> Self {
		Self {
			keep: keep
				.split(',')
				.map(|entry| entry.trim().trim_start_matches('.').to_string())
				.filter(|entry| !entry.is_empty())
				.collect(),
			unlisted,
			authors_sidecar,
		}
	}

	pub fn placement(&self, file: &Path) -> Placement {
		if self.authors_sidecar && named_any(file, AUTHORED_FILES) {
			return Placement::Skip;
		}
		match file.extension().and_then(|extension| extension.to_str()) {
			Some(extension) if is_audio_extension(extension) => Placement::Library,
			Some(extension) if contains_ci(&self.keep, extension) => Placement::Library,
			_ => self.unlisted,
		}
	}
}

fn named_any(file: &Path, candidates: &[&str]) -> bool {
	file.file_name()
		.and_then(|name| name.to_str())
		.is_some_and(|name| {
			candidates
				.iter()
				.any(|known| known.eq_ignore_ascii_case(name))
		})
}

fn contains_ci(values: &[String], target: &str) -> bool {
	values
		.iter()
		.any(|value| value.eq_ignore_ascii_case(target))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::consts::library::DEFAULT_IMPORT_KEEP_EXTENSIONS;

	fn defaults(unlisted: Placement) -> ImportFilter {
		ImportFilter::new(DEFAULT_IMPORT_KEEP_EXTENSIONS, unlisted, false)
	}

	fn placed(filter: &ImportFilter, name: &str) -> Placement {
		filter.placement(Path::new(name))
	}

	#[test]
	fn audio_is_kept_no_matter_what_the_list_says() {
		let filter = ImportFilter::new("", Placement::Skip, false);
		for name in ["01.mp3", "book.M4B", "disc.flac", "part.ogg"] {
			assert_eq!(
				placed(&filter, name),
				Placement::Library,
				"{name} is the content itself, so no list may exclude it"
			);
		}
	}

	#[test]
	fn listed_companions_reach_the_library_and_the_rest_is_skipped() {
		let filter = defaults(Placement::Skip);
		assert_eq!(placed(&filter, "cover.jpg"), Placement::Library);
		assert_eq!(placed(&filter, "book.PDF"), Placement::Library);
		assert_eq!(placed(&filter, "tracker.nfo"), Placement::Skip);
		assert_eq!(placed(&filter, "RARBG.txt"), Placement::Skip);
		assert_eq!(placed(&filter, "proof.url"), Placement::Skip);
	}

	#[test]
	fn the_unlisted_action_decides_where_the_rest_goes() {
		assert_eq!(
			placed(&defaults(Placement::Extras), "tracker.nfo"),
			Placement::Extras
		);
		assert_eq!(
			placed(&defaults(Placement::Library), "tracker.nfo"),
			Placement::Library
		);
	}

	#[test]
	fn files_lunu_writes_itself_are_never_linked_from_the_download() {
		let filter = ImportFilter::new(DEFAULT_IMPORT_KEEP_EXTENSIONS, Placement::Library, true);
		assert_eq!(
			placed(&filter, "metadata.opf"),
			Placement::Skip,
			"a hardlinked copy would make the sidecar write straight into the seeding torrent"
		);
		assert_eq!(placed(&filter, "Cover.JPG"), Placement::Skip);
		assert_eq!(
			placed(&filter, "01.mp3"),
			Placement::Library,
			"only the two files lunu authors are reserved"
		);
	}

	#[test]
	fn a_release_sidecar_survives_when_lunu_is_not_writing_one() {
		let filter = ImportFilter::new(DEFAULT_IMPORT_KEEP_EXTENSIONS, Placement::Skip, false);
		assert_eq!(placed(&filter, "metadata.opf"), Placement::Library);
		assert_eq!(placed(&filter, "cover.jpg"), Placement::Library);
	}

	#[test]
	fn a_list_entry_may_be_written_with_or_without_a_dot_and_in_any_case() {
		let filter = ImportFilter::new(" .JPG , png ", Placement::Skip, false);
		assert_eq!(placed(&filter, "cover.jpg"), Placement::Library);
		assert_eq!(placed(&filter, "back.PNG"), Placement::Library);
	}

	#[test]
	fn a_file_with_no_extension_is_treated_as_unlisted() {
		let filter = defaults(Placement::Skip);
		assert_eq!(placed(&filter, "README"), Placement::Skip);
		assert_eq!(
			placed(&filter, ".nfo"),
			Placement::Skip,
			"a dotfile has no stem, so its name is not an extension"
		);
	}

	#[test]
	fn an_unknown_action_is_rejected_rather_than_guessed() {
		assert!(matches!(
			Placement::from_str("delete"),
			Err(Error::Validation(reason)) if reason == reasons::IMPORT_UNLISTED_ACTION_UNKNOWN
		));
	}
}
