use async_trait::async_trait;

use crate::Result;
use crate::email::RenderedEmail;

#[async_trait]
pub trait Mailer: Send + Sync {
	async fn send(&self, to: &str, email: &RenderedEmail) -> Result<()>;
	async fn send_bulk(
		&self,
		to: &str,
		email: &RenderedEmail,
		unsubscribe: Option<&str>,
	) -> Result<()>;
	async fn is_configured(&self) -> Result<bool>;
	async fn test_connection(&self) -> Result<()>;
}
