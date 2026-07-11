pub const KNOWN_AUDIO_FORMATS: &[&str] =
	&["m4b", "m4a", "flac", "opus", "aac", "ogg", "mp3", "wav"];

pub const DEFAULT_PREFERRED_FORMATS: &[&str] = &["m4b", "m4a", "mp3"];

pub const DEFAULT_MIN_SEEDERS: i64 = 1;
pub const DEFAULT_SEEDER_WEIGHT: i64 = 1;
pub const DEFAULT_FORMAT_WEIGHT: i64 = 100;
