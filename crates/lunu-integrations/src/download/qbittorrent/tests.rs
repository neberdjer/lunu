use super::*;

fn torrents(body: &str) -> Vec<TorrentInfo> {
	serde_json::from_str(body).unwrap()
}

#[test]
fn selects_only_the_torrent_matching_the_requested_hash() {
	let list = torrents(
		r#"[
		{"hash":"aaaa","state":"downloading","progress":0.1,"content_path":"/dl/other"},
		{"hash":"BBBB","state":"uploading","progress":1.0,"content_path":"/dl/mine"}
	]"#,
	);
	let status = select_torrent(list, "bbbb").expect("matches case-insensitively");
	assert_eq!(status.content_path.as_deref(), Some("/dl/mine"));
	assert_eq!(status.state, DownloadState::Completed);
}

#[test]
fn ignores_torrents_when_the_client_does_not_honour_the_hash_filter() {
	let list = torrents(
		r#"[
		{"hash":"aaaa","state":"uploading","progress":1.0,"content_path":"/dl/someone-elses-book"}
	]"#,
	);
	assert!(
		select_torrent(list, "bbbb").is_none(),
		"an unfiltered response must never be mistaken for our torrent: importing it would ingest the wrong files"
	);
}

#[test]
fn no_torrents_means_no_status() {
	assert!(select_torrent(torrents("[]"), "bbbb").is_none());
}

#[test]
fn maps_moving_to_queued_even_at_full_progress() {
	assert_eq!(map_state("moving", 1.0), DownloadState::Queued);
	assert_eq!(map_state("checkingUP", 1.0), DownloadState::Queued);
}

#[test]
fn maps_error_states_to_failed() {
	assert_eq!(map_state("error", 0.5), DownloadState::Failed);
	assert_eq!(map_state("missingFiles", 1.0), DownloadState::Failed);
}

#[test]
fn maps_finished_progress_to_completed() {
	assert_eq!(map_state("uploading", 1.0), DownloadState::Completed);
	assert_eq!(map_state("stalledUP", 1.0), DownloadState::Completed);
}

#[test]
fn maps_in_progress_to_downloading() {
	assert_eq!(map_state("downloading", 0.4), DownloadState::Downloading);
	assert_eq!(map_state("metaDL", 0.0), DownloadState::Downloading);
	assert_eq!(map_state("stalledDL", 0.9), DownloadState::Downloading);
}
