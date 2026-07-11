use async_trait::async_trait;

use crate::Result;
use crate::models::NotificationEvent;

#[async_trait]
pub trait Notifier: Send + Sync {
	fn id(&self) -> &'static str;
	async fn deliver(&self, event: &NotificationEvent) -> Result<()>;
}
