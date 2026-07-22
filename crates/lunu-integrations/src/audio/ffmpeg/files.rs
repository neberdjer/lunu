use std::path::{Path, PathBuf};

use lunu_core::Result;
use lunu_core::services::new_id;

use crate::integration_error;

pub(super) fn backup_shelf(root: &Path, backup: &Path) -> PathBuf {
	root.file_name()
		.map(|name| backup.join(name))
		.unwrap_or_else(|| backup.to_path_buf())
}

pub(super) fn move_all(paths: &[PathBuf], root: &Path, shelf: &Path) -> Result<usize> {
	let mut moved = 0;
	for path in paths {
		let relative = path.strip_prefix(root).unwrap_or(path.as_path());
		let target = shelf.join(relative);
		if let Some(parent) = target.parent() {
			std::fs::create_dir_all(parent).map_err(integration_error)?;
		}
		if std::fs::rename(path, &target).is_err() {
			std::fs::copy(path, &target).map_err(integration_error)?;
			std::fs::remove_file(path).map_err(integration_error)?;
		}
		moved += 1;
	}
	Ok(moved)
}

pub(super) fn remove_all(paths: &[PathBuf]) -> usize {
	paths
		.iter()
		.filter(|path| std::fs::remove_file(path).is_ok())
		.count()
}

pub(super) struct Scratch {
	dir: PathBuf,
}

impl Scratch {
	pub(super) fn create() -> Result<Self> {
		let dir = std::env::temp_dir().join(format!("lunu-merge-{}", new_id()));
		std::fs::create_dir_all(&dir).map_err(integration_error)?;
		Ok(Self { dir })
	}

	pub(super) fn write(&self, name: &str, body: &str) -> Result<PathBuf> {
		let path = self.dir.join(name);
		std::fs::write(&path, body).map_err(integration_error)?;
		Ok(path)
	}
}

impl Drop for Scratch {
	fn drop(&mut self) {
		let _ = std::fs::remove_dir_all(&self.dir);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn scratch_cleans_up_after_itself() {
		let path = {
			let scratch = Scratch::create().unwrap();
			scratch.write("sources.txt", "file 'x'\n").unwrap();
			scratch.dir.clone()
		};
		assert!(!path.exists(), "the scratch directory outlived its merge");
	}
}
