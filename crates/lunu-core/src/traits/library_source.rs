use async_trait::async_trait;

use crate::Result;
use crate::models::LibraryItem;

#[async_trait]
pub trait LibrarySource: Send + Sync {
	async fn list_items(&self) -> Result<Vec<LibraryItem>>;
}
