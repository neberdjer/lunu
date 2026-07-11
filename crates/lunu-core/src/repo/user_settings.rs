use async_trait::async_trait;

use crate::Result;
use crate::models::UserSettings;

#[async_trait]
pub trait UserSettingsRepo: Send + Sync {
	async fn get(&self, user_id: &str) -> Result<Option<UserSettings>>;
	async fn upsert(&self, settings: &UserSettings) -> Result<()>;
	async fn delete(&self, user_id: &str) -> Result<()>;
}
