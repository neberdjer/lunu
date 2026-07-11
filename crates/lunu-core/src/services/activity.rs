use std::sync::Arc;

use chrono::Utc;

use crate::Result;
use crate::models::Activity;
use crate::repo::ActivityRepo;
use crate::services::new_id;
use crate::traits::EventPublisher;

pub struct ActivityService {
	activity: Arc<dyn ActivityRepo>,
	events: Arc<dyn EventPublisher>,
}

impl ActivityService {
	pub fn new(activity: Arc<dyn ActivityRepo>, events: Arc<dyn EventPublisher>) -> Self {
		Self { activity, events }
	}

	pub async fn record(&self, request_id: &str, event: &str) -> Result<()> {
		let activity = Activity {
			id: new_id(),
			request_id: request_id.to_string(),
			event: event.to_string(),
			detail: None,
			at: Utc::now(),
		};
		self.activity.create(&activity).await?;
		self.events.publish(&activity);
		Ok(())
	}

	pub async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<Activity>> {
		self.activity.list_page(limit, offset).await
	}

	pub async fn count(&self) -> Result<i64> {
		self.activity.count().await
	}

	pub async fn for_request(&self, request_id: &str) -> Result<Vec<Activity>> {
		self.activity.for_request(request_id).await
	}
}
