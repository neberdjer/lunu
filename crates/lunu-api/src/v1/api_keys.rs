use actix_web::{HttpResponse, delete, get, post, web};
use lunu_core::consts::auth::KNOWN_API_KEY_SCOPES;
use serde::{Deserialize, Serialize};

use crate::dto::{ApiKeyResponse, IssuedApiKeyResponse};
use crate::error::ApiError;
use crate::extract::AuthUser;
use crate::pagination::{Page, PageParams, Pagination};
use crate::state::AppState;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateApiKeyRequest {
	name: String,
	#[serde(default)]
	scopes: Vec<String>,
	expires_in_days: Option<i64>,
}

#[derive(Serialize, utoipa::ToSchema)]
struct ApiKeyList {
	#[serde(flatten)]
	page: Page<ApiKeyResponse>,
	scopes: &'static [&'static str],
}

#[utoipa::path(tag = "api-keys", params(PageParams), responses((status = 200, description = "Keys plus allowed scopes", body = ApiKeyList)))]
#[get("/api-keys")]
pub async fn list(
	user: AuthUser,
	query: web::Query<PageParams>,
	state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
	let pagination = Pagination::resolve(query.page, query.limit);
	let keys = state
		.api_keys
		.list_for_user_page(&user.id, pagination.limit, pagination.offset)
		.await?;
	let total = state.api_keys.count_for_user(&user.id).await?;
	let items: Vec<ApiKeyResponse> = keys.iter().map(ApiKeyResponse::from).collect();
	Ok(HttpResponse::Ok().json(ApiKeyList {
		page: Page::new(items, &pagination, total),
		scopes: KNOWN_API_KEY_SCOPES,
	}))
}

#[utoipa::path(tag = "api-keys", request_body = CreateApiKeyRequest, responses((status = 201, description = "Key created; secret shown once", body = IssuedApiKeyResponse)))]
#[post("/api-keys")]
pub async fn create(
	user: AuthUser,
	state: web::Data<AppState>,
	body: web::Json<CreateApiKeyRequest>,
) -> Result<HttpResponse, ApiError> {
	let expires_at = crate::expiry::resolve(body.expires_in_days)?;

	let issued = state
		.api_keys
		.issue(&user.id, &body.name, body.scopes.clone(), expires_at)
		.await?;

	Ok(HttpResponse::Created().json(IssuedApiKeyResponse {
		secret: issued.secret,
		api_key: ApiKeyResponse::from(&issued.api_key),
	}))
}

#[utoipa::path(tag = "api-keys", params(("id" = String, Path, description = "API key id")), responses((status = 204, description = "Key revoked")))]
#[delete("/api-keys/{id}")]
pub async fn delete(
	user: AuthUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	state.api_keys.revoke_for_user(&user.id, &id).await?;
	Ok(HttpResponse::NoContent().finish())
}
