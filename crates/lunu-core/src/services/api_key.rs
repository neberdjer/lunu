use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::consts::auth::{API_KEY_DISPLAY_LEN, API_KEY_PREFIX, KNOWN_API_KEY_SCOPES};
use crate::consts::reasons;
use crate::crypto::{generate_token, hash_token};
use crate::models::ApiKey;
use crate::repo::ApiKeyRepo;
use crate::services::new_id;
use crate::{Error, Result};

pub struct IssuedApiKey {
	pub api_key: ApiKey,
	pub secret: String,
}

pub struct ApiKeyService {
	keys: Arc<dyn ApiKeyRepo>,
}

impl ApiKeyService {
	pub fn new(keys: Arc<dyn ApiKeyRepo>) -> Self {
		Self { keys }
	}

	pub async fn issue(
		&self,
		user_id: &str,
		name: &str,
		scopes: Vec<String>,
		expires_at: Option<DateTime<Utc>>,
	) -> Result<IssuedApiKey> {
		if scopes
			.iter()
			.any(|scope| !KNOWN_API_KEY_SCOPES.contains(&scope.as_str()))
		{
			return Err(Error::Validation(reasons::UNKNOWN_SCOPE.to_string()));
		}

		let secret = format!("{API_KEY_PREFIX}_{}", generate_token());
		let prefix = secret.chars().take(API_KEY_DISPLAY_LEN).collect();

		let api_key = ApiKey {
			id: new_id(),
			user_id: user_id.to_string(),
			name: name.to_string(),
			prefix,
			key_hash: hash_token(&secret),
			scopes,
			created_at: Utc::now(),
			last_used_at: None,
			expires_at,
			revoked: false,
		};

		self.keys.create(&api_key).await?;
		Ok(IssuedApiKey { api_key, secret })
	}

	pub async fn list_for_user(&self, user_id: &str) -> Result<Vec<ApiKey>> {
		self.keys.list_for_user(user_id).await
	}

	pub async fn list_for_user_page(
		&self,
		user_id: &str,
		limit: i64,
		offset: i64,
	) -> Result<Vec<ApiKey>> {
		self.keys.list_for_user_page(user_id, limit, offset).await
	}

	pub async fn count_for_user(&self, user_id: &str) -> Result<i64> {
		self.keys.count_for_user(user_id).await
	}

	pub async fn verify(&self, secret: &str) -> Result<Option<ApiKey>> {
		let Some(key) = self.keys.find_by_key_hash(&hash_token(secret)).await? else {
			return Ok(None);
		};

		let now = Utc::now();
		if !key.is_active(now) {
			return Ok(None);
		}

		self.keys.touch_last_used(&key.id, now).await?;
		Ok(Some(key))
	}

	pub async fn revoke(&self, id: &str) -> Result<()> {
		self.keys.set_revoked(id, true).await
	}

	pub async fn revoke_for_user(&self, user_id: &str, id: &str) -> Result<()> {
		if self.keys.revoke_owned(id, user_id).await? {
			Ok(())
		} else {
			Err(Error::NotFound(format!("api key {id}")))
		}
	}
}
