pub(super) fn is_safe_content_path(path: &str) -> bool {
	let trimmed = path.trim();
	if trimmed.is_empty() {
		return false;
	}

	let mut normal = 0;
	for component in std::path::Path::new(trimmed).components() {
		match component {
			std::path::Component::Normal(_) => normal += 1,
			std::path::Component::ParentDir => return false,
			_ => {}
		}
	}
	normal >= 2
}

#[cfg(test)]
mod tests {
	use super::is_safe_content_path;

	#[test]
	fn accepts_a_real_download_path() {
		assert!(is_safe_content_path("/downloads/lunu/The Hobbit"));
		assert!(is_safe_content_path("/data/dl/book.m4b"));
	}

	#[test]
	fn rejects_empty_and_shallow_paths() {
		assert!(!is_safe_content_path(""));
		assert!(!is_safe_content_path("   "));
		assert!(!is_safe_content_path("/"));
		assert!(!is_safe_content_path("/downloads"));
	}

	#[test]
	fn rejects_parent_traversal() {
		assert!(!is_safe_content_path("/downloads/../../etc/passwd"));
		assert!(!is_safe_content_path("/downloads/lunu/../../../root/.ssh"));
		assert!(!is_safe_content_path("../../etc/shadow"));
	}
}
