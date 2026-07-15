use async_trait::async_trait;

use crate::Result;
use crate::models::Invite;

#[async_trait]
pub trait InviteRepo: Send + Sync {
	async fn create(&self, invite: &Invite) -> Result<()>;
	async fn find_by_code_hash(&self, code_hash: &str) -> Result<Option<Invite>>;
	async fn redeem(&self, id: &str) -> Result<bool>;
	async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<Invite>>;
	async fn count(&self) -> Result<i64>;
	async fn delete(&self, id: &str) -> Result<()>;
}
