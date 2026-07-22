use std::path::Path;

use lunu_core::Result;
use lunu_core::consts::merge::COPYABLE_CODEC;
use serde::Deserialize;
use tokio::process::Command;

use crate::integration_error;

pub(super) struct Probed {
	pub(super) seconds: f64,
	pub(super) copyable: bool,
}

#[derive(Deserialize)]
struct ProbeOutput {
	#[serde(default)]
	streams: Vec<ProbeStream>,
	format: Option<ProbeFormat>,
}

#[derive(Deserialize)]
struct ProbeStream {
	codec_name: Option<String>,
}

#[derive(Deserialize)]
struct ProbeFormat {
	duration: Option<String>,
}

pub(super) fn ffprobe_for(ffmpeg: &str) -> String {
	let path = Path::new(ffmpeg);
	let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
		return "ffprobe".to_string();
	};
	let probe = name.replacen("ffmpeg", "ffprobe", 1);
	match path
		.parent()
		.filter(|parent| !parent.as_os_str().is_empty())
	{
		Some(parent) => parent.join(probe).to_string_lossy().into_owned(),
		None => probe,
	}
}

pub(super) async fn probe(ffprobe: &str, path: &Path) -> Result<Probed> {
	let output = Command::new(ffprobe)
		.args([
			"-v",
			"error",
			"-select_streams",
			"a:0",
			"-show_entries",
			"stream=codec_name",
			"-show_entries",
			"format=duration",
			"-of",
			"json",
		])
		.arg(path)
		.output()
		.await
		.map_err(integration_error)?;

	if !output.status.success() {
		return Err(integration_error(format!(
			"ffprobe could not read {}",
			path.display()
		)));
	}

	let parsed: ProbeOutput = serde_json::from_slice(&output.stdout).map_err(integration_error)?;
	from_output(parsed)
		.ok_or_else(|| integration_error(format!("{} reports no duration", path.display())))
}

fn from_output(parsed: ProbeOutput) -> Option<Probed> {
	let copyable = parsed
		.streams
		.into_iter()
		.next()
		.and_then(|stream| stream.codec_name)
		.is_some_and(|codec| codec.eq_ignore_ascii_case(COPYABLE_CODEC));
	let seconds = parsed
		.format
		.and_then(|format| format.duration)
		.and_then(|duration| duration.trim().parse::<f64>().ok())
		.filter(|seconds| *seconds > 0.0)?;
	Some(Probed { seconds, copyable })
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn ffprobe_is_found_beside_whichever_ffmpeg_is_configured() {
		assert_eq!(ffprobe_for("ffmpeg"), "ffprobe");
		assert_eq!(ffprobe_for("/usr/bin/ffmpeg"), "/usr/bin/ffprobe");
		assert_eq!(
			ffprobe_for("/opt/custom/ffmpeg-static"),
			"/opt/custom/ffprobe-static",
			"a suffixed build keeps its suffix so a static pair still resolves"
		);
	}

	fn probed(body: &[u8]) -> Option<Probed> {
		from_output(serde_json::from_slice(body).unwrap())
	}

	#[test]
	fn only_an_aac_source_can_be_copied_rather_than_re_encoded() {
		let mp3 = probed(br#"{"streams":[{"codec_name":"mp3"}],"format":{"duration":"123.45"}}"#)
			.expect("a duration was present");
		assert_eq!(mp3.seconds, 123.45);
		assert!(!mp3.copyable, "mp3 has to be re-encoded to land in an m4b");

		let aac = probed(br#"{"streams":[{"codec_name":"AAC"}],"format":{"duration":"10"}}"#)
			.expect("a duration was present");
		assert!(
			aac.copyable,
			"codec names are matched without regard to case"
		);
	}

	#[test]
	fn a_file_without_a_usable_duration_is_rejected() {
		assert!(probed(br#"{"streams":[],"format":{"duration":"0"}}"#).is_none());
		assert!(probed(br#"{"streams":[],"format":{}}"#).is_none());
	}
}
