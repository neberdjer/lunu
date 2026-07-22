use std::fmt::Write;

use lunu_core::consts::merge::AUDIOBOOK_MEDIA_TYPE;
use lunu_core::traits::PreviewChapter as Chapter;

pub(super) fn concat_list(paths: &[std::path::PathBuf]) -> String {
	paths
		.iter()
		.map(|path| format!("file '{}'\n", path.to_string_lossy().replace('\'', "'\\''")))
		.collect()
}

pub(super) struct Tags<'a> {
	pub(super) title: &'a str,
	pub(super) author: Option<&'a str>,
	pub(super) series: Option<&'a str>,
	pub(super) sequence: Option<&'a str>,
}

pub(super) fn ffmetadata(tags: &Tags<'_>, chapters: &[Chapter]) -> String {
	let title = escape_value(tags.title);
	let mut body = format!(";FFMETADATA1\ntitle={title}\n");
	if let Some(author) = tags.author {
		let author = escape_value(author);
		let _ = writeln!(body, "artist={author}\nalbum_artist={author}");
	}
	let _ = writeln!(body, "album={title}");
	let _ = writeln!(body, "media_type={AUDIOBOOK_MEDIA_TYPE}");
	if let Some(series) = tags.series.map(escape_value) {
		let _ = writeln!(body, "show={series}");
		match tags.sequence.map(escape_value) {
			Some(sequence) => {
				let _ = writeln!(body, "episode_id={sequence}\ngrouping={series} #{sequence}");
			}
			None => {
				let _ = writeln!(body, "grouping={series}");
			}
		}
	}

	let mut start_ms: i64 = 0;
	for chapter in chapters {
		let end_ms = start_ms + (chapter.seconds * 1000.0).round() as i64;
		let _ = write!(
			body,
			"\n[CHAPTER]\nTIMEBASE=1/1000\nSTART={start_ms}\nEND={end_ms}\ntitle={}\n",
			escape_value(&chapter.title)
		);
		start_ms = end_ms;
	}
	body
}

fn escape_value(value: &str) -> String {
	value
		.replace('\\', "\\\\")
		.replace('=', "\\=")
		.replace(';', "\\;")
		.replace('#', "\\#")
		.replace('\n', " ")
}

#[cfg(test)]
mod tests {
	use std::path::PathBuf;

	use super::*;

	#[test]
	fn chapters_run_back_to_back_from_zero() {
		let body = ffmetadata(
			&Tags {
				title: "The Hobbit",
				author: Some("Tolkien"),
				series: Some("Middle-earth"),
				sequence: Some("1"),
			},
			&[
				Chapter {
					title: "One".to_string(),
					seconds: 60.0,
				},
				Chapter {
					title: "Two".to_string(),
					seconds: 30.5,
				},
			],
		);
		assert!(body.starts_with(";FFMETADATA1\n"));
		assert!(body.contains("title=The Hobbit"));
		assert!(body.contains("artist=Tolkien"));
		assert!(
			body.contains("media_type=2"),
			"without stik=2 players treat a merged audiobook as music and forget its position"
		);
		assert!(
			body.contains("show=Middle-earth") && body.contains("episode_id=1"),
			"audiobookshelf reads series from show and sequence from episode_id: {body}"
		);
		assert!(body.contains("START=0\nEND=60000\ntitle=One"));
		assert!(
			body.contains("START=60000\nEND=90500\ntitle=Two"),
			"the second chapter must start exactly where the first ended: {body}"
		);
	}

	#[test]
	fn metadata_values_cannot_break_out_of_their_line() {
		let body = ffmetadata(
			&Tags {
				title: "A=B;C#D",
				author: None,
				series: None,
				sequence: None,
			},
			&[],
		);
		assert!(body.contains("title=A\\=B\\;C\\#D"));
	}

	#[test]
	fn a_quoted_path_survives_the_concat_list() {
		let list = concat_list(&[PathBuf::from("/lib/Rock 'n' Roll/01.mp3")]);
		assert_eq!(list, "file '/lib/Rock '\\''n'\\'' Roll/01.mp3'\n");
	}
}
