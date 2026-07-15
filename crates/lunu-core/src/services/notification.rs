use std::sync::Arc;

use crate::models::{NotificationEvent, NotificationKind};
use crate::repo::UserRepo;
use crate::traits::Notifier;
use crate::{Error, Result};

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

pub struct DispatchReport {
	pub delivered: usize,
	pub failed: usize,
	pub last_error: Option<Error>,
}

impl DispatchReport {
	pub fn total_failure(&self) -> bool {
		self.delivered == 0 && self.failed > 0
	}
}

pub struct NotificationService {
	notifiers: Vec<Arc<dyn Notifier>>,
}

impl NotificationService {
	pub fn new(notifiers: Vec<Arc<dyn Notifier>>) -> Self {
		Self { notifiers }
	}

	pub async fn dispatch(&self, event: &NotificationEvent) -> Result<DispatchReport> {
		let mut report = DispatchReport {
			delivered: 0,
			failed: 0,
			last_error: None,
		};
		for notifier in &self.notifiers {
			match notifier.deliver(event).await {
				Ok(()) => report.delivered += 1,
				Err(error) => {
					report.failed += 1;
					report.last_error = Some(error);
				}
			}
		}
		Ok(report)
	}
}
