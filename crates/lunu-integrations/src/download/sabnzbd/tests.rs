use super::*;

const ADD_OK: &str = r#"{"status": true, "nzo_ids": ["SABnzbd_nzo_kjkq9g"]}"#;
const ADD_BAD_KEY: &str = r#"{"status": false, "error": "API Key Incorrect"}"#;

const QUEUE: &str = r#"{
	"queue": {
		"slots": [
			{"nzo_id": "SABnzbd_nzo_kjkq9g", "status": "Downloading", "percentage": "34"},
			{"nzo_id": "SABnzbd_nzo_paused", "status": "Paused", "percentage": "0"}
		]
	}
}"#;

const HISTORY: &str = r#"{
	"history": {
		"slots": [
			{"nzo_id": "SABnzbd_nzo_done", "status": "Completed", "storage": "/downloads/lunu/Book"},
			{"nzo_id": "SABnzbd_nzo_bad", "status": "Failed", "fail_message": "unpack error"}
		]
	}
}"#;

#[test]
fn an_add_response_carries_the_nzo_id() {
	let added: AddResponse = serde_json::from_str(ADD_OK).unwrap();
	assert!(added.status);
	assert_eq!(added.nzo_ids, vec!["SABnzbd_nzo_kjkq9g"]);
}

#[test]
fn a_rejected_add_carries_the_error() {
	let added: AddResponse = serde_json::from_str(ADD_BAD_KEY).unwrap();
	assert!(!added.status);
	assert_eq!(added.error.as_deref(), Some("API Key Incorrect"));
}

#[test]
fn a_queue_slot_maps_percentage_and_state() {
	let queue: QueueResponse = serde_json::from_str(QUEUE).unwrap();
	let slots = queue.queue.unwrap().slots;

	let downloading = queue_status(&slots[0]);
	assert_eq!(downloading.state, DownloadState::Downloading);
	assert!((downloading.progress - 0.34).abs() < f64::EPSILON);
	assert_eq!(downloading.content_path, None);

	let paused = queue_status(&slots[1]);
	assert_eq!(paused.state, DownloadState::Queued);
}

#[test]
fn history_completion_carries_the_storage_path() {
	let history: HistoryResponse = serde_json::from_str(HISTORY).unwrap();
	let mut slots = history.history.unwrap().slots.into_iter();

	let done = history_status(slots.next().unwrap());
	assert_eq!(done.state, DownloadState::Completed);
	assert!((done.progress - 1.0).abs() < f64::EPSILON);
	assert_eq!(done.content_path.as_deref(), Some("/downloads/lunu/Book"));

	let failed = history_status(slots.next().unwrap());
	assert_eq!(failed.state, DownloadState::Failed);
}

#[test]
fn a_checking_queue_slot_is_still_queued() {
	let slot = QueueSlot {
		nzo_id: "x".to_string(),
		status: "Checking".to_string(),
		percentage: "0".to_string(),
	};
	assert_eq!(queue_status(&slot).state, DownloadState::Queued);
}

#[test]
fn post_processing_happens_in_history_and_counts_as_downloading() {
	for status in ["Verifying", "Repairing", "Extracting", "Moving", "Running"] {
		let slot = HistorySlot {
			nzo_id: "x".to_string(),
			status: status.to_string(),
			storage: None,
		};
		let mapped = history_status(slot);
		assert_eq!(
			mapped.state,
			DownloadState::Downloading,
			"{status} is not terminal, so the monitor must keep polling"
		);
		assert_eq!(mapped.content_path, None);
	}
}
