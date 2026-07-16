use super::*;

#[test]
fn an_added_or_duplicate_torrent_yields_its_hash() {
	let added = serde_json::json!({
		"torrent-added": { "hashString": "abc123", "id": 7, "name": "Book" }
	});
	assert_eq!(added_hash(&added).as_deref(), Some("abc123"));

	let duplicate = serde_json::json!({
		"torrent-duplicate": { "hashString": "abc123", "id": 7, "name": "Book" }
	});
	assert_eq!(added_hash(&duplicate).as_deref(), Some("abc123"));

	assert_eq!(added_hash(&serde_json::json!({})), None);
}

#[test]
fn torrent_states_map_to_download_states() {
	let torrent = |status: i64, percent_done: f64| TorrentInfo {
		status,
		percent_done,
		download_dir: "/downloads/lunu".to_string(),
		name: "Book".to_string(),
	};

	assert_eq!(
		map_torrent(torrent(4, 0.5)).state,
		DownloadState::Downloading
	);
	assert_eq!(map_torrent(torrent(3, 0.0)).state, DownloadState::Queued);
	assert_eq!(map_torrent(torrent(2, 0.1)).state, DownloadState::Queued);
	assert_eq!(map_torrent(torrent(6, 1.0)).state, DownloadState::Completed);
	assert_eq!(
		map_torrent(torrent(0, 1.0)).state,
		DownloadState::Completed,
		"a stopped torrent that finished is complete, not stuck"
	);
	assert_eq!(map_torrent(torrent(0, 0.4)).state, DownloadState::Queued);
}

#[test]
fn the_content_path_appears_only_once_complete() {
	let done = map_torrent(TorrentInfo {
		status: 6,
		percent_done: 1.0,
		download_dir: "/downloads/lunu".to_string(),
		name: "Book".to_string(),
	});
	assert_eq!(done.content_path.as_deref(), Some("/downloads/lunu/Book"));

	let partial = map_torrent(TorrentInfo {
		status: 4,
		percent_done: 0.5,
		download_dir: "/downloads/lunu".to_string(),
		name: "Book".to_string(),
	});
	assert_eq!(
		partial.content_path, None,
		"an importer must never see a half-written directory"
	);
}

#[test]
fn the_rpc_endpoint_tolerates_both_url_shapes() {
	assert_eq!(
		rpc_endpoint("http://host:9091"),
		"http://host:9091/transmission/rpc"
	);
	assert_eq!(
		rpc_endpoint("http://host:9091/transmission/rpc/"),
		"http://host:9091/transmission/rpc"
	);
}
