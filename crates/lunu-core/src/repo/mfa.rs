use async_trait::async_trait;

use crate::Result;
use crate::models::{MfaRecoveryCode, UserMfa};

#[async_trait]
pub trait UserMfaRepo: Send + Sync {
	async fn upsert(&self, mfa: &UserMfa) -> Result<()>;
	async fn find_for_user(&self, user_id: &str) -> Result<Option<UserMfa>>;
	async fn delete_for_user(&self, user_id: &str) -> Result<()>;
}

#[async_trait]
pub trait MfaRecoveryCodeRepo: Send + Sync {
	async fn replace_for_user(&self, user_id: &str, codes: &[MfaRecoveryCode]) -> Result<()>;
	async fn consume(&self, user_id: &str, code_hash: &str) -> Result<bool>;
	async fn count_unused(&self, user_id: &str) -> Result<i64>;
	async fn delete_for_user(&self, user_id: &str) -> Result<()>;
}
