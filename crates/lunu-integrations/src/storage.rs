use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

pub struct DiskUsage {
	pub free_bytes: u64,
	pub total_bytes: u64,
}

pub fn disk_usage(path: &Path) -> Option<DiskUsage> {
	let c_path = CString::new(path.as_os_str().as_bytes()).ok()?;
	let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
	if unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) } != 0 {
		return None;
	}
	let block = stat.f_frsize as u64;
	Some(DiskUsage {
		free_bytes: block.saturating_mul(stat.f_bavail as u64),
		total_bytes: block.saturating_mul(stat.f_blocks as u64),
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn an_existing_mount_reports_a_nonzero_total() {
		let usage = disk_usage(Path::new("/")).expect("root is always mounted");
		assert!(usage.total_bytes > 0, "a real filesystem has a size");
		assert!(
			usage.free_bytes <= usage.total_bytes,
			"free cannot exceed total"
		);
	}

	#[test]
	fn a_path_that_does_not_exist_reports_nothing_rather_than_zero() {
		assert!(disk_usage(Path::new("/no/such/path/lunu-readiness")).is_none());
	}
}
