use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::consts::merge::{
	DEFAULT_FFMPEG_BINARY, MERGE_PROBE_CONCURRENCY, MERGE_SKIP_ALREADY_MERGED,
	MERGE_SKIP_NOT_MULTI_FILE, MERGE_SKIP_OUTPUT_EXISTS, SETTING_MERGE_FFMPEG_PATH,
};
use lunu_core::models::SourceDisposition;
use lunu_core::services::SettingsService;
use lunu_core::traits::{
	MergeOutcome, MergePlan, MergePreview, MergeSummary, Merger, PreviewChapter, RevertPlan,
};
use tokio::process::Command;

mod files;
mod metadata;
mod probe;
mod sources;

use crate::integration_error;
use files::{Scratch, backup_shelf, move_all, remove_all};

const PARTIAL_SUFFIX: &str = ".part";
const M4B_MUXER: &str = "ipod";

pub struct FfmpegMerger {
	settings: Arc<SettingsService>,
}

struct Ready {
	output: PathBuf,
	sources: Vec<PathBuf>,
	chapters: Vec<PreviewChapter>,
	copyable: bool,
}

enum Prepared {
	Skip(&'static str),
	Ready(Box<Ready>),
}

impl FfmpegMerger {
	pub fn new(settings: Arc<SettingsService>) -> Self {
		Self { settings }
	}

	async fn binary(&self) -> Result<String> {
		Ok(self
			.settings
			.get_or_default(SETTING_MERGE_FFMPEG_PATH)
			.await?
			.unwrap_or_else(|| DEFAULT_FFMPEG_BINARY.to_string()))
	}

	async fn prepare(&self, plan: &MergePlan, binary: &str) -> Result<Prepared> {
		let previous = plan.previous_output.as_ref().map(PathBuf::from);
		if previous.as_deref().is_some_and(Path::exists) {
			return Ok(Prepared::Skip(MERGE_SKIP_ALREADY_MERGED));
		}

		let output = PathBuf::from(&plan.output_path);
		if output.exists() {
			return Ok(Prepared::Skip(MERGE_SKIP_OUTPUT_EXISTS));
		}

		let sources = {
			let dir = PathBuf::from(&plan.source_dir);
			let exclude: Vec<PathBuf> = std::iter::once(output.clone()).chain(previous).collect();
			tokio::task::spawn_blocking(move || sources::collect(&dir, &exclude))
				.await
				.map_err(integration_error)??
		};
		if sources.len() < 2 {
			return Ok(Prepared::Skip(MERGE_SKIP_NOT_MULTI_FILE));
		}

		let ffprobe = probe::ffprobe_for(binary);
		let mut chapters = Vec::with_capacity(sources.len());
		let mut copyable = true;
		for batch in sources.chunks(MERGE_PROBE_CONCURRENCY) {
			let probing: Vec<_> = batch
				.iter()
				.map(|path| {
					let ffprobe = ffprobe.clone();
					let path = path.clone();
					tokio::spawn(async move { probe::probe(&ffprobe, &path).await })
				})
				.collect();
			for (path, probed) in batch.iter().zip(probing) {
				let probed = probed.await.map_err(integration_error)??;
				copyable &= probed.copyable;
				chapters.push(PreviewChapter {
					title: sources::chapter_title(path),
					seconds: probed.seconds,
				});
			}
		}

		Ok(Prepared::Ready(Box::new(Ready {
			output,
			sources,
			chapters,
			copyable,
		})))
	}
}

#[async_trait]
impl Merger for FfmpegMerger {
	async fn test_connection(&self) -> Result<()> {
		let binary = self.binary().await?;
		let reachable = Command::new(&binary)
			.arg("-version")
			.output()
			.await
			.is_ok_and(|output| output.status.success());
		if reachable {
			return Ok(());
		}
		Err(lunu_core::Error::Validation(
			lunu_core::consts::reasons::MERGE_UNAVAILABLE.to_string(),
		))
	}

	async fn preview(&self, plan: &MergePlan) -> Result<MergePreview> {
		let ready = match self.prepare(plan, &self.binary().await?).await? {
			Prepared::Skip(reason) => return Ok(MergePreview::skipped(reason)),
			Prepared::Ready(ready) => *ready,
		};
		Ok(MergePreview {
			total_seconds: ready.chapters.iter().map(|chapter| chapter.seconds).sum(),
			copyable: ready.copyable,
			chapters: ready.chapters,
			..MergePreview::of(plan)
		})
	}

	async fn merge(&self, plan: &MergePlan) -> Result<MergeOutcome> {
		let binary = self.binary().await?;
		let ready = match self.prepare(plan, &binary).await? {
			Prepared::Skip(reason) => return Ok(MergeOutcome::Skipped(reason)),
			Prepared::Ready(ready) => *ready,
		};

		let scratch = Scratch::create()?;
		let list = scratch.write("sources.txt", &metadata::concat_list(&ready.sources))?;
		let tags = scratch.write(
			"chapters.txt",
			&metadata::ffmetadata(
				&metadata::Tags {
					title: &plan.title,
					author: plan.author.as_deref(),
					series: plan.series.as_deref(),
					sequence: plan.sequence.as_deref(),
				},
				&ready.chapters,
			),
		)?;

		let partial = PathBuf::from(format!("{}{PARTIAL_SUFFIX}", plan.output_path));
		let encode = (!ready.copyable).then_some(plan.bitrate.as_str());
		run_ffmpeg(&binary, &list, &tags, &partial, encode).await?;
		std::fs::rename(&partial, &ready.output).map_err(integration_error)?;

		let chapter_count = ready.chapters.len();
		let root = PathBuf::from(&plan.source_dir);
		let shelf = plan
			.sources
			.backup()
			.map(|backup| backup_shelf(&root, Path::new(backup)));
		let deletes = plan.sources == SourceDisposition::Delete;
		let destination = shelf.clone();
		let sources = ready.sources;
		let handled = tokio::task::spawn_blocking(move || match destination {
			Some(shelf) => move_all(&sources, &root, &shelf),
			None if deletes => Ok(remove_all(&sources)),
			None => Ok(0),
		})
		.await
		.map_err(integration_error)??;

		Ok(MergeOutcome::Merged(MergeSummary {
			output_path: plan.output_path.clone(),
			backup_path: shelf.map(|path| path.to_string_lossy().into_owned()),
			chapters: chapter_count,
			handled_sources: handled,
		}))
	}

	async fn revert(&self, plan: &RevertPlan) -> Result<usize> {
		let shelf = PathBuf::from(&plan.backup_path);
		let root = PathBuf::from(&plan.source_dir);
		let merged = PathBuf::from(&plan.merged_path);

		tokio::task::spawn_blocking(move || {
			let mut shelved = Vec::new();
			sources::walk_all(&shelf, &mut shelved)?;
			let restored = move_all(&shelved, &shelf, &root)?;
			std::fs::remove_file(&merged).map_err(integration_error)?;
			let _ = std::fs::remove_dir_all(&shelf);
			Ok(restored)
		})
		.await
		.map_err(integration_error)?
	}
}

async fn run_ffmpeg(
	binary: &str,
	list: &Path,
	tags: &Path,
	destination: &Path,
	encode: Option<&str>,
) -> Result<()> {
	let mut command = Command::new(binary);
	command.kill_on_drop(true);
	command
		.args(["-nostdin", "-y", "-f", "concat", "-safe", "0", "-i"])
		.arg(list)
		.args(["-f", "ffmetadata", "-i"])
		.arg(tags)
		.args(["-map", "0:a", "-map_metadata", "1"]);

	match encode {
		None => command.args(["-c:a", "copy"]),
		Some(bitrate) => command.args(["-c:a", "aac", "-b:a", bitrate]),
	};

	let output = command
		.args(["-movflags", "+faststart", "-f", M4B_MUXER])
		.arg(destination)
		.output()
		.await
		.map_err(integration_error)?;

	if !output.status.success() {
		let _ = std::fs::remove_file(destination);
		let detail = String::from_utf8_lossy(&output.stderr);
		let tail: String = detail.lines().rev().take(3).collect::<Vec<_>>().join("; ");
		return Err(integration_error(format!("ffmpeg merge failed: {tail}")));
	}
	Ok(())
}
