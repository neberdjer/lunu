use std::sync::Arc;

use crate::Result;
use crate::models::{NotificationEvent, NotificationKind};
use crate::repo::UserRepo;
use crate::traits::Notifier;

pub async fn resolve_recipients(
	users: &dyn UserRepo,
	event: &NotificationEvent,
) -> Result<Vec<String>> {
	if event.kind == NotificationKind::RequestPending {
		users.enabled_admin_ids().await
	} else {
		Ok(vec![event.user_id.clone()])
	}
}

pub struct NotificationService {
	notifiers: Vec<Arc<dyn Notifier>>,
}

impl NotificationService {
	pub fn new(notifiers: Vec<Arc<dyn Notifier>>) -> Self {
		Self { notifiers }
	}

	pub async fn dispatch(&self, event: &NotificationEvent) -> Result<()> {
		let mut last_error = None;
		for notifier in &self.notifiers {
			if let Err(error) = notifier.deliver(event).await {
				last_error = Some(error);
			}
		}
		last_error.map_or(Ok(()), Err)
	}
}
