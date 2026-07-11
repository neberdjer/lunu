use lunu_core::consts::api::WS_EVENT_BUFFER;
use lunu_core::models::Activity;
use lunu_core::traits::EventPublisher;
use tokio::sync::broadcast;

pub struct EventHub {
	tx: broadcast::Sender<Activity>,
}

impl EventHub {
	pub fn new() -> Self {
		let (tx, _rx) = broadcast::channel(WS_EVENT_BUFFER);
		Self { tx }
	}

	pub fn subscribe(&self) -> broadcast::Receiver<Activity> {
		self.tx.subscribe()
	}
}

impl Default for EventHub {
	fn default() -> Self {
		Self::new()
	}
}

impl EventPublisher for EventHub {
	fn publish(&self, activity: &Activity) {
		let _ = self.tx.send(activity.clone());
	}
}
