use crate::models::Activity;

pub trait EventPublisher: Send + Sync {
	fn publish(&self, activity: &Activity);
}
