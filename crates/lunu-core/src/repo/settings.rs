use async_trait::async_trait;

use crate::Result;
use crate::models::Setting;

#[async_trait]
pub trait SettingsRepo: Send + Sync {
	async fn get(&self, key: &str) -> Result<Option<Setting>>;
	async fn set(&self, setting: &Setting) -> Result<()>;
	async fn get_all(&self) -> Result<Vec<Setting>>;
	async fn delete(&self, key: &str) -> Result<()>;
}
