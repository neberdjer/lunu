use std::sync::Arc;

use chrono::Utc;

use crate::Result;
use crate::models::Activity;
use crate::repo::ActivityRepo;
use crate::services::new_id;

pub struct ActivityService {
	activity: Arc<dyn ActivityRepo>,
}

impl ActivityService {
	pub fn new(activity: Arc<dyn ActivityRepo>) -> Self {
		Self { activity }
	}

	pub async fn record(&self, request_id: &str, event: &str) -> Result<()> {
		let activity = Activity {
			id: new_id(),
			request_id: request_id.to_string(),
			event: event.to_string(),
			detail: None,
			at: Utc::now(),
		};
		self.activity.create(&activity).await
	}

	pub async fn recent(&self, limit: i64) -> Result<Vec<Activity>> {
		self.activity.recent(limit).await
	}

	pub async fn for_request(&self, request_id: &str) -> Result<Vec<Activity>> {
		self.activity.for_request(request_id).await
	}
}
