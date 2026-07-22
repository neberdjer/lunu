pub const KNOWN_AUDIO_FORMATS: &[&str] =
	&["m4b", "m4a", "flac", "opus", "aac", "ogg", "mp3", "wav"];

pub const DEFAULT_PREFERRED_FORMATS: &[&str] = &["m4b", "m4a", "mp3"];

pub const DEFAULT_AVOIDED_KEYWORDS: &[&str] = &["abridged"];

pub const DEFAULT_MIN_SEEDERS: i64 = 1;
pub const DEFAULT_SEEDER_WEIGHT: i64 = 1;
pub const DEFAULT_FORMAT_WEIGHT: i64 = 100;
pub const DEFAULT_KEYWORD_WEIGHT: i64 = 100;
pub const DEFAULT_PROTOCOL_WEIGHT: i64 = 100;
pub const DEFAULT_BITRATE_WEIGHT: i64 = 1;
pub const DEFAULT_FREELEECH_WEIGHT: i64 = 0;

pub const MIN_PLAUSIBLE_KBPS: i64 = 8;
pub const MAX_PLAUSIBLE_KBPS: i64 = 320;

pub const FREELEECH_TOKENS: &[&str] = &["freeleech", "free leech", "freeleach", "fl"];

pub const LANGUAGE_TOKENS: &[(&str, &[&str])] = &[
	("en", &["english", "eng"]),
	("de", &["german", "deutsch", "ger", "deu"]),
	("fr", &["french", "francais", "fre", "fra"]),
	("es", &["spanish", "espanol", "spa", "esp"]),
	("it", &["italian", "italiano", "ita"]),
	("nl", &["dutch", "nederlands", "nld"]),
	("pt", &["portuguese", "portugues", "por"]),
	("ru", &["russian", "rus"]),
	("pl", &["polish", "polski", "pol"]),
	("sv", &["swedish", "svenska", "swe"]),
];
