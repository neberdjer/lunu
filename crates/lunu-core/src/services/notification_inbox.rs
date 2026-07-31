use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::Result;
use crate::consts::jobs::NOTIFICATION_RETENTION_DAYS;
use crate::models::{LiveEvent, NotificationEvent, UserNotification};
use crate::repo::{UserNotificationRepo, UserRepo};
use crate::services::{new_id, resolve_recipients};
use crate::traits::EventPublisher;

pub struct NotificationInboxService {
	repo: Arc<dyn UserNotificationRepo>,
	users: Arc<dyn UserRepo>,
	events: Arc<dyn EventPublisher>,
}

impl NotificationInboxService {
	pub fn new(
		repo: Arc<dyn UserNotificationRepo>,
		users: Arc<dyn UserRepo>,
		events: Arc<dyn EventPublisher>,
	) -> Self {
		Self {
			repo,
			users,
			events,
		}
	}

	pub async fn fan_out(&self, event: &NotificationEvent) -> Result<()> {
		let mut last_error = None;
		for user_id in resolve_recipients(self.users.as_ref(), event).await? {
			if let Err(error) = self.deliver(&user_id, event).await {
				last_error = Some(error);
			}
		}
		last_error.map_or(Ok(()), Err)
	}

	async fn deliver(&self, user_id: &str, event: &NotificationEvent) -> Result<()> {
		let notification = UserNotification {
			id: new_id(),
			user_id: user_id.to_string(),
			kind: event.kind,
			request_id: Some(event.request_id.clone()),
			title: event.title.clone(),
			created_at: Utc::now(),
			read_at: None,
		};
		self.repo.create(&notification).await?;
		self.events.publish(&LiveEvent::Notification(notification));
		Ok(())
	}

	pub async fn list(
		&self,
		user_id: &str,
		limit: i64,
		offset: i64,
	) -> Result<Vec<UserNotification>> {
		self.repo.list_for_user(user_id, limit, offset).await
	}

	pub async fn count(&self, user_id: &str) -> Result<i64> {
		self.repo.count_for_user(user_id).await
	}

	pub async fn unread_count(&self, user_id: &str) -> Result<i64> {
		self.repo.unread_count(user_id).await
	}

	pub async fn mark_read(&self, user_id: &str, id: &str) -> Result<bool> {
		self.repo.mark_read(user_id, id, Utc::now()).await
	}

	pub async fn mark_all_read(&self, user_id: &str) -> Result<u64> {
		self.repo.mark_all_read(user_id, Utc::now()).await
	}

	pub async fn prune_old(&self) -> Result<u64> {
		let cutoff = Utc::now() - Duration::days(NOTIFICATION_RETENTION_DAYS);
		self.repo.delete_before(cutoff).await
	}
}
