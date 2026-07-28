use crate::dto::{
	IntegrationOkResponse, SettingSpecResponse, SettingViewResponse, SettingsCatalogResponse,
};
use actix_web::{HttpResponse, delete, get, post, put, web};
use lunu_core::Error;
use lunu_core::consts::settings;
use serde::Deserialize;

use crate::error::ApiError;
use crate::extract::AdminUser;
use crate::state::AppState;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct SetSettingRequest {
	value: String,
}

#[utoipa::path(tag = "settings", responses((status = 200, description = "Set keys plus the settings catalog", body = SettingsCatalogResponse)))]
#[get("/settings")]
pub async fn list(_admin: AdminUser, state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
	let keys = state.settings.keys().await?;
	let catalog: Vec<SettingSpecResponse> = settings::registry()
		.map(|spec| SettingSpecResponse {
			key: spec.key.to_string(),
			kind: spec.kind.as_str().to_string(),
			choices: spec.kind.choices().iter().map(|c| c.to_string()).collect(),
			secret: spec.secret,
			default: spec.default.map(str::to_string),
		})
		.collect();
	Ok(HttpResponse::Ok().json(SettingsCatalogResponse { keys, catalog }))
}

#[utoipa::path(tag = "settings", params(("key" = String, Path, description = "Setting key")), responses((status = 200, description = "Setting value (masked if secret)", body = SettingViewResponse), (status = 404, description = "Unknown key")))]
#[get("/settings/{key}")]
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

	Ok(HttpResponse::Ok().json(SettingViewResponse {
		key,
		secret: view.secret,
		value: view.value,
	}))
}

#[utoipa::path(tag = "settings", params(("key" = String, Path, description = "Setting key")), request_body = SetSettingRequest, responses((status = 204, description = "Setting stored")))]
#[put("/settings/{key}")]
pub async fn set(
	_admin: AdminUser,
	state: web::Data<AppState>,
	key: web::Path<String>,
	body: web::Json<SetSettingRequest>,
) -> Result<HttpResponse, ApiError> {
	state.settings.set(&key, &body.value).await?;
	Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(tag = "settings", params(("key" = String, Path, description = "Setting key")), responses((status = 204, description = "Setting cleared")))]
#[delete("/settings/{key}")]
pub async fn delete(
	_admin: AdminUser,
	state: web::Data<AppState>,
	key: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	state.settings.delete(&key).await?;
	Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(tag = "settings", params(("integration" = String, Path, description = "Integration name")), responses((status = 200, description = "Integration reachable", body = IntegrationOkResponse), (status = 404, description = "Unknown integration")))]
#[post("/settings/test/{integration}")]
pub async fn test(
	_admin: AdminUser,
	state: web::Data<AppState>,
	integration: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let integration = integration.into_inner();
	match integration.as_str() {
		settings::PROWLARR => state.releases.test_indexer().await,
		settings::FFMPEG => state.merges.test().await,
		_ => state.grabs.test_download(&integration).await,
	}?;
	Ok(HttpResponse::Ok().json(IntegrationOkResponse { ok: true }))
}
