use actix_web::{HttpResponse, web};
use lunu_core::Error;
use lunu_core::consts::settings;
use serde::Deserialize;
use serde_json::json;

use crate::error::ApiError;
use crate::extract::AdminUser;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct SetSettingRequest {
	value: String,
}

pub async fn list(_admin: AdminUser, state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
	let keys = state.settings.keys().await?;
	let catalog: Vec<_> = settings::REGISTRY
		.iter()
		.map(|spec| {
			json!({
				"key": spec.key,
				"kind": spec.kind.as_str(),
				"choices": spec.kind.choices(),
				"secret": spec.secret,
				"default": spec.default,
			})
		})
		.collect();
	Ok(HttpResponse::Ok().json(json!({ "keys": keys, "catalog": catalog })))
}

pub async fn get(
	_admin: AdminUser,
	state: web::Data<AppState>,
	key: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let key = key.into_inner();
	let view = state
		.settings
		.view(&key)
		.await?
		.ok_or_else(|| Error::NotFound(format!("setting {key}")))?;

	Ok(HttpResponse::Ok().json(json!({ "key": key, "secret": view.secret, "value": view.value })))
}

pub async fn set(
	_admin: AdminUser,
	state: web::Data<AppState>,
	key: web::Path<String>,
	body: web::Json<SetSettingRequest>,
) -> Result<HttpResponse, ApiError> {
	state.settings.set(&key, &body.value).await?;
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

pub async fn test(
	_admin: AdminUser,
	state: web::Data<AppState>,
	integration: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let integration = integration.into_inner();
	match integration.as_str() {
		settings::PROWLARR => state.releases.test_indexer().await,
		settings::QBITTORRENT => state.grabs.test_download().await,
		_ => return Err(Error::NotFound(format!("integration {integration}")).into()),
	}?;
	Ok(HttpResponse::Ok().json(json!({ "ok": true })))
}
