pub const SETTING_MERGE_ENABLED: &str = "merge_enabled";
pub const SETTING_MERGE_FFMPEG_PATH: &str = "merge_ffmpeg_path";
pub const SETTING_MERGE_SOURCE_ACTION: &str = "merge_source_action";
pub const SETTING_MERGE_BACKUP_DIR: &str = "merge_backup_dir";

pub const MERGE_SOURCE_MOVE: &str = "move";
pub const MERGE_SOURCE_DELETE: &str = "delete";
pub const MERGE_SOURCE_KEEP: &str = "keep";
pub const MERGE_SOURCE_ACTIONS: &[&str] =
	&[MERGE_SOURCE_MOVE, MERGE_SOURCE_DELETE, MERGE_SOURCE_KEEP];
pub const DEFAULT_MERGE_SOURCE_ACTION: &str = MERGE_SOURCE_MOVE;
pub const SETTING_MERGE_BITRATE: &str = "merge_bitrate";

pub const DEFAULT_FFMPEG_BINARY: &str = "ffmpeg";
pub const DEFAULT_MERGE_BITRATE: &str = "64k";

pub const MERGE_OUTPUT_EXTENSION: &str = "m4b";
pub const AUDIOBOOK_MEDIA_TYPE: &str = "2";
pub const COPYABLE_CODEC: &str = "aac";

pub const MERGE_SKIP_NO_LIBRARY_PATH: &str = "no-library-path";
pub const MERGE_SKIP_ALREADY_MERGED: &str = "already-merged";
pub const MERGE_SKIP_OUTPUT_EXISTS: &str = "output-exists";
pub const MERGE_SKIP_NOT_MULTI_FILE: &str = "not-multi-file";
pub const MERGE_SKIP_REASONS: &[&str] = &[
	MERGE_SKIP_NO_LIBRARY_PATH,
	MERGE_SKIP_ALREADY_MERGED,
	MERGE_SKIP_OUTPUT_EXISTS,
	MERGE_SKIP_NOT_MULTI_FILE,
];

pub const ACTIVITY_MERGED: &str = "merged";
pub const ACTIVITY_MERGE_SKIPPED: &str = "merge-skipped";
pub const ACTIVITY_MERGE_REVERTED: &str = "merge-reverted";

pub const MERGE_ALL_LIMIT: i64 = 500;
pub const MERGE_PROBE_CONCURRENCY: usize = 8;
