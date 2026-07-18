use async_trait::async_trait;

use crate::Result;
use crate::models::UserMfa;

#[async_trait]
pub trait UserMfaRepo: Send + Sync {
	async fn upsert(&self, mfa: &UserMfa) -> Result<()>;
	async fn find_for_user(&self, user_id: &str) -> Result<Option<UserMfa>>;
	async fn delete_for_user(&self, user_id: &str) -> Result<()>;
}
