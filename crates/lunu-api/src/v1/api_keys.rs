use actix_web::{HttpResponse, web};
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::json;

use crate::dto::ApiKeyResponse;
use crate::error::ApiError;
use crate::extract::AuthUser;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateApiKeyRequest {
	name: String,
	#[serde(default)]
	scopes: Vec<String>,
	expires_in_days: Option<i64>,
}

pub async fn list(user: AuthUser, state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
	let keys = state.api_keys.list_for_user(&user.id).await?;
	let response: Vec<ApiKeyResponse> = keys.iter().map(ApiKeyResponse::from).collect();
	Ok(HttpResponse::Ok().json(response))
}

pub async fn create(
	user: AuthUser,
	state: web::Data<AppState>,
	body: web::Json<CreateApiKeyRequest>,
) -> Result<HttpResponse, ApiError> {
	let expires_at = body
		.expires_in_days
		.map(|days| Utc::now() + Duration::days(days));

	let issued = state
		.api_keys
		.issue(&user.id, &body.name, body.scopes.clone(), expires_at)
		.await?;

	Ok(HttpResponse::Created().json(json!({
		"secret": issued.secret,
		"api_key": ApiKeyResponse::from(&issued.api_key),
	})))
}

pub async fn delete(
	user: AuthUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	state.api_keys.revoke_for_user(&user.id, &id).await?;
	Ok(HttpResponse::NoContent().finish())
}
