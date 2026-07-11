use async_trait::async_trait;

use crate::Result;

#[async_trait]
pub trait Importer: Send + Sync {
	async fn import(&self, source: &str, destination: &str) -> Result<()>;
}
