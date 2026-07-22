use async_trait::async_trait;

use crate::Result;
use crate::models::ImportFilter;

#[async_trait]
pub trait Importer: Send + Sync {
	async fn import(&self, source: &str, destination: &str, filter: &ImportFilter) -> Result<()>;
}
