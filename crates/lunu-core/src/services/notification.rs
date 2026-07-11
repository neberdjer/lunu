use std::sync::Arc;

use crate::Result;
use crate::models::NotificationEvent;
use crate::traits::Notifier;

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
