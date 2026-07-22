use std::fs;
use std::path::Path;

use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::consts::library::EXTRAS_DIR;
use lunu_core::models::{ImportFilter, Placement};
use lunu_core::traits::Importer;

use crate::integration_error;

#[derive(Default)]
pub struct HardlinkImporter;

impl HardlinkImporter {
	pub fn new() -> Self {
		Self
	}
}

#[async_trait]
impl Importer for HardlinkImporter {
	async fn import(&self, source: &str, destination: &str, filter: &ImportFilter) -> Result<()> {
		let source = source.to_string();
		let destination = destination.to_string();
		let filter = filter.clone();
		tokio::task::spawn_blocking(move || {
			import_path(Path::new(&source), Path::new(&destination), &filter)
		})
		.await
		.map_err(integration_error)?
	}
}

fn import_path(source: &Path, destination: &Path, filter: &ImportFilter) -> Result<()> {
	let metadata = fs::metadata(source).map_err(integration_error)?;
	if metadata.is_dir() {
		fs::create_dir_all(destination).map_err(integration_error)?;
		return import_dir(source, destination, Path::new(""), filter);
	}
	let name = source
		.file_name()
		.ok_or_else(|| integration_error(format!("source has no file name: {source:?}")))?;
	place_filtered(source, destination, Path::new(name), filter)
}

fn import_dir(
	source: &Path,
	destination: &Path,
	relative: &Path,
	filter: &ImportFilter,
) -> Result<()> {
	for entry in fs::read_dir(source).map_err(integration_error)? {
		let entry = entry.map_err(integration_error)?;
		let path = entry.path();
		let relative = relative.join(entry.file_name());
		if entry.file_type().map_err(integration_error)?.is_dir() {
			import_dir(&path, destination, &relative, filter)?;
		} else {
			place_filtered(&path, destination, &relative, filter)?;
		}
	}
	Ok(())
}

fn place_filtered(
	source: &Path,
	destination: &Path,
	relative: &Path,
	filter: &ImportFilter,
) -> Result<()> {
	match filter.placement(relative) {
		Placement::Skip => Ok(()),
		Placement::Library => place_file(source, &destination.join(relative)),
		Placement::Extras => place_file(source, &destination.join(EXTRAS_DIR).join(relative)),
	}
}

fn place_file(source: &Path, target: &Path) -> Result<()> {
	if let Some(parent) = target.parent() {
		fs::create_dir_all(parent).map_err(integration_error)?;
	}
	if fs::hard_link(source, target).is_ok() {
		return Ok(());
	}
	fs::copy(source, target).map_err(integration_error)?;
	Ok(())
}

#[cfg(test)]
mod tests {
	use std::sync::atomic::{AtomicU32, Ordering};

	use super::*;

	static COUNTER: AtomicU32 = AtomicU32::new(0);

	fn keep_all() -> ImportFilter {
		ImportFilter::new("", Placement::Library, false)
	}

	fn temp_dir() -> std::path::PathBuf {
		let unique = format!(
			"lunu-import-{}-{}",
			std::process::id(),
			COUNTER.fetch_add(1, Ordering::Relaxed)
		);
		let dir = std::env::temp_dir().join(unique);
		fs::create_dir_all(&dir).unwrap();
		dir
	}

	#[tokio::test]
	async fn imports_a_single_file() {
		let root = temp_dir();
		let source = root.join("book.m4b");
		fs::write(&source, b"audio").unwrap();
		let dest = root.join("library/Author/Title");

		HardlinkImporter::new()
			.import(
				source.to_str().unwrap(),
				dest.to_str().unwrap(),
				&keep_all(),
			)
			.await
			.unwrap();

		let placed = dest.join("book.m4b");
		assert_eq!(fs::read(&placed).unwrap(), b"audio");
		fs::remove_dir_all(&root).unwrap();
	}

	#[tokio::test]
	async fn imports_a_directory_tree() {
		let root = temp_dir();
		let source = root.join("Book");
		fs::create_dir_all(source.join("disc1")).unwrap();
		fs::write(source.join("disc1/01.mp3"), b"one").unwrap();
		fs::write(source.join("cover.jpg"), b"img").unwrap();
		let dest = root.join("library/Author/Title");

		HardlinkImporter::new()
			.import(
				source.to_str().unwrap(),
				dest.to_str().unwrap(),
				&keep_all(),
			)
			.await
			.unwrap();

		assert_eq!(fs::read(dest.join("disc1/01.mp3")).unwrap(), b"one");
		assert_eq!(fs::read(dest.join("cover.jpg")).unwrap(), b"img");
		fs::remove_dir_all(&root).unwrap();
	}

	#[tokio::test]
	async fn unlisted_files_stay_out_of_the_library_and_out_of_the_download_dir_untouched() {
		let root = temp_dir();
		let source = root.join("Book");
		fs::create_dir_all(source.join("disc1")).unwrap();
		fs::write(source.join("disc1/01.mp3"), b"one").unwrap();
		fs::write(source.join("cover.jpg"), b"img").unwrap();
		fs::write(source.join("tracker.nfo"), b"ad").unwrap();
		let dest = root.join("library/Author/Title");

		let filter = ImportFilter::new("jpg", Placement::Skip, false);
		HardlinkImporter::new()
			.import(source.to_str().unwrap(), dest.to_str().unwrap(), &filter)
			.await
			.unwrap();

		assert_eq!(fs::read(dest.join("disc1/01.mp3")).unwrap(), b"one");
		assert_eq!(fs::read(dest.join("cover.jpg")).unwrap(), b"img");
		assert!(
			!dest.join("tracker.nfo").exists(),
			"an unlisted file must not reach the library"
		);
		assert!(
			source.join("tracker.nfo").exists(),
			"skipping must never delete from the download directory, which is still seeding"
		);
		fs::remove_dir_all(&root).unwrap();
	}

	#[tokio::test]
	async fn the_extras_action_files_the_rest_beside_the_book_rather_than_dropping_it() {
		let root = temp_dir();
		let source = root.join("Book");
		fs::create_dir_all(source.join("scans")).unwrap();
		fs::write(source.join("01.mp3"), b"one").unwrap();
		fs::write(source.join("scans/back.tiff"), b"scan").unwrap();
		let dest = root.join("library/Author/Title");

		let filter = ImportFilter::new("jpg", Placement::Extras, false);
		HardlinkImporter::new()
			.import(source.to_str().unwrap(), dest.to_str().unwrap(), &filter)
			.await
			.unwrap();

		assert_eq!(fs::read(dest.join("01.mp3")).unwrap(), b"one");
		assert_eq!(
			fs::read(dest.join("extras/scans/back.tiff")).unwrap(),
			b"scan",
			"extras keep their original folder structure so nothing collides"
		);
		fs::remove_dir_all(&root).unwrap();
	}
}
