use async_trait::async_trait;

use crate::Result;
use crate::models::EmailVerificationToken;

#[async_trait]
pub trait EmailVerificationRepo: Send + Sync {
	async fn create(&self, token: &EmailVerificationToken) -> Result<()>;
	async fn find_for_user(&self, user_id: &str) -> Result<Option<EmailVerificationToken>>;
	async fn increment_attempts(&self, id: &str) -> Result<()>;
	async fn delete(&self, id: &str) -> Result<()>;
	async fn delete_for_user(&self, user_id: &str) -> Result<()>;
}
