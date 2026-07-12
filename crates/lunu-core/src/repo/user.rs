use async_trait::async_trait;

use crate::Result;
use crate::models::User;

#[async_trait]
pub trait UserRepo: Send + Sync {
	async fn create(&self, user: &User) -> Result<()>;
	async fn create_initial_admin(&self, user: &User) -> Result<bool>;
	async fn count_enabled_admins_excluding(&self, id: &str) -> Result<i64>;
	async fn update(&self, user: &User) -> Result<()>;
	async fn find_by_id(&self, id: &str) -> Result<Option<User>>;
	async fn find_by_username(&self, username: &str) -> Result<Option<User>>;
	async fn find_by_email(&self, email: &str) -> Result<Option<User>>;
	async fn list(&self) -> Result<Vec<User>>;
	async fn enabled_admin_ids(&self) -> Result<Vec<String>>;
	async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<User>>;
	async fn count(&self) -> Result<i64>;
	async fn delete(&self, id: &str) -> Result<()>;
}
