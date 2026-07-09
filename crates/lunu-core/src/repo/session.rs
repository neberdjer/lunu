use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::Result;
use crate::models::Session;

#[async_trait]
pub trait SessionRepo: Send + Sync {
	async fn create(&self, session: &Session) -> Result<()>;
	async fn find_by_token_hash(&self, token_hash: &str) -> Result<Option<Session>>;
	async fn touch(&self, id: &str, last_seen_at: DateTime<Utc>) -> Result<()>;
	async fn delete(&self, id: &str) -> Result<()>;
	async fn delete_for_user(&self, user_id: &str) -> Result<()>;
	async fn delete_expired(&self, now: DateTime<Utc>) -> Result<()>;
}
