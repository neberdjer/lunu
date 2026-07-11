use std::fs;
use std::path::Path;

use async_trait::async_trait;
use lunu_core::Result;
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
	async fn import(&self, source: &str, destination: &str) -> Result<()> {
		let source = source.to_string();
		let destination = destination.to_string();
		tokio::task::spawn_blocking(move || {
			import_path(Path::new(&source), Path::new(&destination))
		})
		.await
		.map_err(integration_error)?
	}
}

fn import_path(source: &Path, destination: &Path) -> Result<()> {
	let metadata = fs::metadata(source).map_err(integration_error)?;
	if metadata.is_dir() {
		import_dir(source, destination)
	} else {
		let name = source
			.file_name()
			.ok_or_else(|| integration_error(format!("source has no file name: {source:?}")))?;
		place_file(source, &destination.join(name))
	}
}

fn import_dir(source: &Path, destination: &Path) -> Result<()> {
	fs::create_dir_all(destination).map_err(integration_error)?;
	for entry in fs::read_dir(source).map_err(integration_error)? {
		let entry = entry.map_err(integration_error)?;
		let path = entry.path();
		let target = destination.join(entry.file_name());
		if entry.file_type().map_err(integration_error)?.is_dir() {
			import_dir(&path, &target)?;
		} else {
			place_file(&path, &target)?;
		}
	}
	Ok(())
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
			.import(source.to_str().unwrap(), dest.to_str().unwrap())
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
			.import(source.to_str().unwrap(), dest.to_str().unwrap())
			.await
			.unwrap();

		assert_eq!(fs::read(dest.join("disc1/01.mp3")).unwrap(), b"one");
		assert_eq!(fs::read(dest.join("cover.jpg")).unwrap(), b"img");
		fs::remove_dir_all(&root).unwrap();
	}
}
