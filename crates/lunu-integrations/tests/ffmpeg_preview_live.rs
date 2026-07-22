mod common;

use std::path::PathBuf;

use common::merge::{merged, merger, plan, probe_seconds, scratch, skipped, write_tone};

use lunu_core::consts::merge::MERGE_SKIP_NOT_MULTI_FILE;
use lunu_core::models::SourceDisposition;
use lunu_core::traits::Merger;

#[tokio::test]
#[ignore]
async fn a_preview_reports_what_the_merge_then_actually_does() {
	let dir = scratch("preview");
	write_tone(&dir.join("01 - The Shire.mp3"), 2);
	write_tone(&dir.join("02 - Rivendell.mp3"), 3);

	let plan = plan(&dir, SourceDisposition::Keep);
	let preview = merger().preview(&plan).await.expect("preview succeeds");

	assert!(preview.skip.is_none(), "this folder is genuinely mergeable");
	assert_eq!(preview.chapters.len(), 2);
	assert_eq!(
		preview
			.chapters
			.iter()
			.map(|chapter| chapter.title.as_str())
			.collect::<Vec<_>>(),
		["01 - The Shire", "02 - Rivendell"],
		"the preview lists the chapters in the order they would be written"
	);
	assert!(
		(preview.total_seconds - 5.0).abs() < 0.5,
		"got {}",
		preview.total_seconds
	);
	assert!(
		!preview.copyable,
		"mp3 sources need a re-encode, which is the slow answer the user wants in advance"
	);
	assert!(
		dir.join("01 - The Shire.mp3").exists() && !PathBuf::from(&preview.output_path).exists(),
		"a preview must not touch a single file"
	);

	let summary = merged(merger().merge(&plan).await.unwrap());
	assert_eq!(
		summary.chapters,
		preview.chapters.len(),
		"the merge must produce exactly the chapter count the preview promised"
	);
	assert_eq!(summary.output_path, preview.output_path);
	assert!(
		(probe_seconds(&PathBuf::from(&summary.output_path)) - preview.total_seconds).abs() < 0.5,
		"the runtime the preview predicted must be the runtime the merge produced"
	);
	std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
#[ignore]
async fn a_preview_predicts_a_skip_rather_than_discovering_it_during_the_merge() {
	let dir = scratch("preview-skip");
	write_tone(&dir.join("only.mp3"), 1);

	let plan = plan(&dir, SourceDisposition::Keep);
	let preview = merger().preview(&plan).await.unwrap();

	assert_eq!(preview.skip, Some(MERGE_SKIP_NOT_MULTI_FILE));
	assert_eq!(
		skipped(merger().merge(&plan).await.unwrap()),
		preview.skip.unwrap(),
		"preview and merge must reach the same verdict, or the preview is a lie"
	);
	std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
#[ignore]
async fn many_sources_keep_their_order_through_concurrent_probing() {
	let dir = scratch("probe-order");
	for track in 1..=24 {
		write_tone(&dir.join(format!("{track:02} - Track.mp3")), 1);
	}

	let preview = merger()
		.preview(&plan(&dir, SourceDisposition::Keep))
		.await
		.unwrap();

	let titles: Vec<String> = preview
		.chapters
		.into_iter()
		.map(|chapter| chapter.title)
		.collect();
	let expected: Vec<String> = (1..=24)
		.map(|track| format!("{track:02} - Track"))
		.collect();
	assert_eq!(
		titles, expected,
		"chapters are probed in parallel batches, so a lost ordering would silently scramble \
		 the chapter list against the audio"
	);
	std::fs::remove_dir_all(&dir).unwrap();
}
