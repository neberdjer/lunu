use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::Utc;
use lunu_core::models::SourceDisposition;
use lunu_core::traits::{MergeOutcome, MergePlan, MergeSummary};
use lunu_integrations::audio::FfmpegMerger;

pub fn merger() -> FfmpegMerger {
	FfmpegMerger::new(super::no_settings())
}

pub fn scratch(name: &str) -> PathBuf {
	let dir = std::env::temp_dir().join(format!(
		"lunu-merge-live-{}-{name}-{}",
		std::process::id(),
		Utc::now().timestamp_nanos_opt().unwrap_or_default()
	));
	std::fs::create_dir_all(&dir).unwrap();
	dir
}

pub fn write_tone(path: &Path, seconds: u32) {
	let status = Command::new("ffmpeg")
		.args(["-nostdin", "-y", "-f", "lavfi", "-i"])
		.arg(format!("sine=frequency=440:duration={seconds}"))
		.args(["-c:a", "libmp3lame", "-b:a", "32k"])
		.arg(path)
		.output()
		.expect("ffmpeg generates a tone");
	assert!(status.status.success(), "could not build the fixture audio");
}

pub fn probe_seconds(path: &Path) -> f64 {
	let out = Command::new("ffprobe")
		.args([
			"-v",
			"error",
			"-show_entries",
			"format=duration",
			"-of",
			"default=nw=1:nk=1",
		])
		.arg(path)
		.output()
		.unwrap();
	String::from_utf8_lossy(&out.stdout).trim().parse().unwrap()
}

pub fn merged(outcome: MergeOutcome) -> MergeSummary {
	match outcome {
		MergeOutcome::Merged(summary) => summary,
		MergeOutcome::Skipped(reason) => panic!("expected a merge, it was skipped: {reason}"),
	}
}

pub fn skipped(outcome: MergeOutcome) -> &'static str {
	match outcome {
		MergeOutcome::Skipped(reason) => reason,
		MergeOutcome::Merged(_) => panic!("expected a skip, it merged"),
	}
}

pub fn tag(path: &Path, key: &str) -> String {
	let out = Command::new("ffprobe")
		.args([
			"-v",
			"error",
			"-show_entries",
			&format!("format_tags={key}"),
			"-of",
			"default=nw=1:nk=1",
		])
		.arg(path)
		.output()
		.unwrap();
	String::from_utf8_lossy(&out.stdout).trim().to_string()
}

pub fn chapter_count(path: &Path) -> usize {
	let out = Command::new("ffprobe")
		.args(["-v", "error", "-show_chapters", "-of", "json"])
		.arg(path)
		.output()
		.unwrap();
	let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
	parsed["chapters"].as_array().map(Vec::len).unwrap_or(0)
}

pub fn plan(dir: &Path, sources: SourceDisposition) -> MergePlan {
	MergePlan {
		source_dir: dir.to_string_lossy().into_owned(),
		output_path: dir.join("The Hobbit.m4b").to_string_lossy().into_owned(),
		title: "The Hobbit".to_string(),
		author: Some("Tolkien".to_string()),
		series: Some("Middle-earth".to_string()),
		sequence: Some("1".to_string()),
		bitrate: "32k".to_string(),
		sources,
		previous_output: None,
	}
}
