use crate::models::LiveEvent;

pub trait EventPublisher: Send + Sync {
	fn publish(&self, event: &LiveEvent);
}
