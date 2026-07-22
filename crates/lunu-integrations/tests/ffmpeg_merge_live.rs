mod common;

use lunu_core::consts::merge::{MERGE_SKIP_NOT_MULTI_FILE, MERGE_SKIP_OUTPUT_EXISTS};

use std::path::PathBuf;

use common::merge::scratch;
use common::merge::{chapter_count, merged, merger, plan, probe_seconds, skipped, tag, write_tone};
use lunu_core::models::SourceDisposition;
use lunu_core::traits::{Merger, RevertPlan};

#[tokio::test]
#[ignore]
async fn three_mp3s_become_one_chaptered_m4b() {
	let dir = scratch("chapters");
	write_tone(&dir.join("01 - The Shire.mp3"), 2);
	write_tone(&dir.join("02 - Rivendell.mp3"), 3);
	write_tone(&dir.join("10 - The Mountain.mp3"), 1);

	let summary = merged(
		merger()
			.merge(&plan(&dir, SourceDisposition::Keep))
			.await
			.expect("merge succeeds"),
	);

	let output = PathBuf::from(&summary.output_path);
	assert!(output.exists(), "the merged m4b was not written");
	assert_eq!(summary.chapters, 3);
	assert_eq!(
		chapter_count(&output),
		3,
		"ffmpeg must record one chapter per source file"
	);
	let seconds = probe_seconds(&output);
	assert!(
		(seconds - 6.0).abs() < 0.5,
		"the merged runtime must be the sum of its parts, got {seconds}"
	);
	assert!(
		dir.join("01 - The Shire.mp3").exists(),
		"sources survive unless replacement was asked for"
	);
	assert_eq!(
		tag(&output, "media_type"),
		"2",
		"stik=2 is what makes a player treat this as an audiobook rather than music"
	);
	assert_eq!(tag(&output, "show"), "Middle-earth", "series must survive");
	assert_eq!(tag(&output, "episode_id"), "1", "sequence must survive");
	assert_eq!(tag(&output, "album"), "The Hobbit");
	std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
#[ignore]
async fn deleting_sources_leaves_only_the_m4b() {
	let dir = scratch("replace");
	write_tone(&dir.join("1.mp3"), 1);
	write_tone(&dir.join("2.mp3"), 1);

	let summary = merged(
		merger()
			.merge(&plan(&dir, SourceDisposition::Delete))
			.await
			.unwrap(),
	);

	assert_eq!(summary.handled_sources, 2);
	assert!(!dir.join("1.mp3").exists());
	assert!(!dir.join("2.mp3").exists());
	assert!(PathBuf::from(&summary.output_path).exists());
	std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
#[ignore]
async fn a_single_file_and_an_already_merged_folder_are_both_left_alone() {
	let dir = scratch("noop");
	write_tone(&dir.join("only.mp3"), 1);
	assert_eq!(
		skipped(
			merger()
				.merge(&plan(&dir, SourceDisposition::Keep))
				.await
				.unwrap()
		),
		MERGE_SKIP_NOT_MULTI_FILE,
		"one file is not a multi-file release"
	);

	write_tone(&dir.join("second.mp3"), 1);
	merged(
		merger()
			.merge(&plan(&dir, SourceDisposition::Keep))
			.await
			.unwrap(),
	);
	assert_eq!(
		skipped(
			merger()
				.merge(&plan(&dir, SourceDisposition::Keep))
				.await
				.unwrap()
		),
		MERGE_SKIP_OUTPUT_EXISTS,
		"an existing m4b means the work is already done, and must never be re-merged into itself"
	);
	std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
#[ignore]
async fn moving_sources_shelves_them_in_the_backup_directory() {
	let dir = scratch("move");
	let backup = scratch("backup");
	write_tone(&dir.join("01.mp3"), 1);
	write_tone(&dir.join("02.mp3"), 1);

	let summary = merged(
		merger()
			.merge(&plan(
				&dir,
				SourceDisposition::Move {
					backup: backup.to_string_lossy().into_owned(),
				},
			))
			.await
			.unwrap(),
	);

	assert_eq!(summary.handled_sources, 2);
	assert!(
		!dir.join("01.mp3").exists(),
		"the scanned folder must hold only the merged file"
	);
	assert!(PathBuf::from(&summary.output_path).exists());

	let shelf = backup.join(dir.file_name().unwrap());
	assert!(
		shelf.join("01.mp3").exists() && shelf.join("02.mp3").exists(),
		"originals must be recoverable from the backup directory, not destroyed"
	);
	std::fs::remove_dir_all(&dir).unwrap();
	std::fs::remove_dir_all(&backup).unwrap();
}

#[tokio::test]
#[ignore]
async fn reverting_a_merge_restores_the_originals_and_removes_the_m4b() {
	let dir = scratch("revert");
	let backup = scratch("revert-backup");
	write_tone(&dir.join("01 - The Shire.mp3"), 1);
	write_tone(&dir.join("02 - Rivendell.mp3"), 1);

	let summary = merged(
		merger()
			.merge(&plan(
				&dir,
				SourceDisposition::Move {
					backup: backup.to_string_lossy().into_owned(),
				},
			))
			.await
			.unwrap(),
	);
	let shelf = summary
		.backup_path
		.clone()
		.expect("a move reports its shelf");
	assert_eq!(
		PathBuf::from(&shelf),
		backup.join(dir.file_name().unwrap()),
		"the summary must name the folder the originals actually landed in"
	);

	let restored = merger()
		.revert(&RevertPlan {
			source_dir: dir.to_string_lossy().into_owned(),
			merged_path: summary.output_path.clone(),
			backup_path: shelf.clone(),
		})
		.await
		.expect("revert succeeds");

	assert_eq!(restored, 2);
	assert!(
		dir.join("01 - The Shire.mp3").exists() && dir.join("02 - Rivendell.mp3").exists(),
		"every shelved original must come back to where it was"
	);
	assert!(
		!PathBuf::from(&summary.output_path).exists(),
		"the m4b must go, or the library would show the book twice"
	);
	assert!(
		!PathBuf::from(&shelf).exists(),
		"an emptied shelf must not linger in the backup directory"
	);
	std::fs::remove_dir_all(&dir).unwrap();
	let _ = std::fs::remove_dir_all(&backup);
}
