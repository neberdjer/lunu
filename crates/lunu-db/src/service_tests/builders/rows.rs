use super::*;

pub(crate) fn request(id: &str) -> Request {
	let now = Utc::now();
	Request {
		id: id.to_string(),
		user_id: "u1".to_string(),
		work_id: format!("work-{id}"),
		format: Format::Audiobook,
		asin: Some(id.to_string()),
		title: "t".to_string(),
		author: None,
		cover_url: None,
		series_name: None,
		series_sequence: None,
		status: RequestStatus::Pending,
		approved_by: None,
		notes: None,
		quality_profile_id: None,
		created_at: now,
		updated_at: now,
	}
}

pub(crate) fn hobbit() -> Request {
	Request {
		work_id: "work-B01".to_string(),
		asin: Some("B01".to_string()),
		title: "The Hobbit".to_string(),
		cover_url: Some("https://covers/hobbit.jpg".to_string()),
		..request("r1")
	}
}

pub(crate) fn media(id: &str) -> Media {
	Media {
		id: id.to_string(),
		work_id: None,
		format: Format::Audiobook,
		asin: None,
		abs_item_id: None,
		title: "t".to_string(),
		author: None,
		cover_url: None,
		series_name: None,
		series_sequence: None,
		library_path: String::new(),
		merged_path: None,
		merge_state: MergeState::default(),
		merge_detail: None,
		merge_backup_path: None,
		source: MediaSource::Request,
		overridden: false,
		matched_by: None,
		request_id: None,
		created_at: Utc::now(),
	}
}

pub(crate) async fn request_status(db: &Db) -> RequestStatus {
	SqlxRequestRepo::new(db.clone())
		.find_by_id("r1")
		.await
		.unwrap()
		.unwrap()
		.status
}
