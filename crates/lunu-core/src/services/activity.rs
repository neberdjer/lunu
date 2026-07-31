use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::Result;
use crate::consts::jobs::ACTIVITY_RETENTION_DAYS;
use crate::models::{Activity, ActivityTarget, LiveEvent};
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

	pub async fn record(
		&self,
		target: ActivityTarget<'_>,
		event: &str,
		detail: Option<&str>,
		actor: Option<&str>,
	) -> Result<()> {
		let (request_id, media_id) = match target {
			ActivityTarget::Request(id) => (Some(id.to_string()), None),
			ActivityTarget::Media(id) => (None, Some(id.to_string())),
		};
		let activity = Activity {
			id: new_id(),
			request_id,
			media_id,
			event: event.to_string(),
			detail: detail.map(str::to_string),
			actor: actor.map(str::to_string),
			at: Utc::now(),
		};
		self.activity.create(&activity).await?;
		self.events.publish(&LiveEvent::Activity(activity));
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

	pub async fn delete_for_request(&self, request_id: &str) -> Result<()> {
		self.activity.delete_for_request(request_id).await
	}

	pub async fn prune_old(&self) -> Result<u64> {
		let cutoff = Utc::now() - Duration::days(ACTIVITY_RETENTION_DAYS);
		self.activity.delete_before(cutoff).await
	}
}
