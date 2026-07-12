use async_trait::async_trait;

use crate::Result;

#[async_trait]
pub trait Mailer: Send + Sync {
	async fn send(&self, to: &str, subject: &str, html: &str) -> Result<()>;
}
