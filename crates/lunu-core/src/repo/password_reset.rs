use async_trait::async_trait;

use crate::Result;
use crate::models::PasswordResetToken;

#[async_trait]
pub trait PasswordResetRepo: Send + Sync {
	async fn create(&self, token: &PasswordResetToken) -> Result<()>;
	async fn find_for_user(&self, user_id: &str) -> Result<Option<PasswordResetToken>>;
	async fn claim_attempt(&self, id: &str, max_attempts: i64) -> Result<bool>;
	async fn delete(&self, id: &str) -> Result<()>;
	async fn delete_for_user(&self, user_id: &str) -> Result<()>;
}
