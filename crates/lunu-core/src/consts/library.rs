pub const SETTING_LIBRARY_DIR: &str = "library_dir";
pub const UNKNOWN_AUTHOR: &str = "Unknown Author";
pub const MATCH_CONFIDENCE_FLOOR: f64 = 0.85;

pub const METADATA_OPF_FILE: &str = "metadata.opf";
pub const COVER_FILE: &str = "cover.jpg";
pub const SETTING_WRITE_SIDECAR: &str = "import_write_metadata";

pub const SETTING_IMPORT_KEEP_EXTENSIONS: &str = "import_keep_extensions";
pub const DEFAULT_IMPORT_KEEP_EXTENSIONS: &str = "jpg,jpeg,png,opf,cue,pdf,epub";

pub const SETTING_IMPORT_UNLISTED: &str = "import_unlisted_files";
pub const IMPORT_UNLISTED_SKIP: &str = "skip";
pub const IMPORT_UNLISTED_EXTRAS: &str = "extras";
pub const IMPORT_UNLISTED_KEEP: &str = "keep";
pub const IMPORT_UNLISTED_ACTIONS: &[&str] = &[
	IMPORT_UNLISTED_SKIP,
	IMPORT_UNLISTED_EXTRAS,
	IMPORT_UNLISTED_KEEP,
];
pub const DEFAULT_IMPORT_UNLISTED: &str = IMPORT_UNLISTED_SKIP;
pub const EXTRAS_DIR: &str = "extras";
