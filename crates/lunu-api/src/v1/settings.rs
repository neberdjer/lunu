use actix_web::{HttpResponse, web};
use lunu_core::Error;
use serde::Deserialize;
use serde_json::json;

use crate::error::ApiError;
use crate::extract::AdminUser;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct SetSettingRequest {
	value: String,
	#[serde(default)]
	secret: bool,
}

pub async fn list(_admin: AdminUser, state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
	let keys = state.settings.keys().await?;
	Ok(HttpResponse::Ok().json(json!({ "keys": keys })))
}

pub async fn get(
	_admin: AdminUser,
	state: web::Data<AppState>,
	key: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let key = key.into_inner();
	let value = state
		.settings
		.get(&key)
		.await?
		.ok_or_else(|| Error::NotFound(format!("setting {key}")))?;

	Ok(HttpResponse::Ok().json(json!({ "key": key, "value": value })))
}

pub async fn set(
	_admin: AdminUser,
	state: web::Data<AppState>,
	key: web::Path<String>,
	body: web::Json<SetSettingRequest>,
) -> Result<HttpResponse, ApiError> {
	state.settings.set(&key, &body.value, body.secret).await?;
	Ok(HttpResponse::NoContent().finish())
}

pub async fn delete(
	_admin: AdminUser,
	state: web::Data<AppState>,
	key: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	state.settings.delete(&key).await?;
	Ok(HttpResponse::NoContent().finish())
}
