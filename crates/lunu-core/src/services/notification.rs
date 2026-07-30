use std::collections::HashSet;
use std::sync::Arc;

use crate::models::{NotificationEvent, NotificationKind};
use crate::repo::{NotificationDeliveryRepo, UserRepo};
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

pub struct NotificationService {
	notifiers: Vec<Arc<dyn Notifier>>,
	deliveries: Arc<dyn NotificationDeliveryRepo>,
}

impl NotificationService {
	pub fn new(
		notifiers: Vec<Arc<dyn Notifier>>,
		deliveries: Arc<dyn NotificationDeliveryRepo>,
	) -> Self {
		Self {
			notifiers,
			deliveries,
		}
	}

	pub async fn dispatch(
		&self,
		job_id: &str,
		event: &NotificationEvent,
	) -> Result<DispatchReport> {
		let already: HashSet<String> = self
			.deliveries
			.delivered_channels(job_id)
			.await?
			.into_iter()
			.collect();
		let mut report = DispatchReport {
			delivered: already.len(),
			failed: 0,
			last_error: None,
		};

		let mut sending = tokio::task::JoinSet::new();
		for notifier in &self.notifiers {
			if already.contains(notifier.id()) {
				continue;
			}
			let notifier = notifier.clone();
			let event = event.clone();
			let channel = notifier.id().to_string();
			sending.spawn(async move { (channel, notifier.deliver(&event).await) });
		}

		let mut delivered = Vec::new();
		while let Some(finished) = sending.join_next().await {
			match finished {
				Ok((channel, Ok(()))) => {
					report.delivered += 1;
					delivered.push(channel);
				}
				Ok((_, Err(error))) => {
					report.failed += 1;
					report.last_error = Some(error);
				}
				Err(error) => {
					report.failed += 1;
					report.last_error = Some(Error::Internal(format!(
						"notification task did not complete: {error}"
					)));
				}
			}
		}

		if report.failed > 0 {
			for channel in delivered {
				self.deliveries.record(job_id, &channel).await?;
			}
		} else if !already.is_empty() {
			self.deliveries.clear(job_id).await?;
		}
		Ok(report)
	}

	pub async fn prune_orphaned_deliveries(&self) -> Result<u64> {
		self.deliveries.prune_orphaned().await
	}
}
