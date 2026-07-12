use std::str::FromStr;

use actix_web::{HttpResponse, delete, get, patch, post, put, web};
use lunu_core::models::{Role, UserSettings};
use serde::Deserialize;

use crate::dto::{UserResponse, UserSettingsResponse};
use crate::error::ApiError;
use crate::extract::AdminUser;
use crate::pagination::{Page, PageParams, Pagination};
use crate::state::AppState;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateUserRequest {
	username: String,
	password: String,
	email: Option<String>,
	role: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateUserRequest {
	#[serde(default)]
	enabled: Option<bool>,
	#[serde(default)]
	role: Option<String>,
	#[serde(default)]
	display_name: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct SetPasswordRequest {
	password: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct SetUserSettingsRequest {
	auto_approve: bool,
	request_quota: Option<i64>,
	quota_days: Option<i64>,
}

#[utoipa::path(tag = "users", params(PageParams), responses((status = 200, body = Page<UserResponse>)))]
#[get("/users")]
pub async fn list(
	_admin: AdminUser,
	query: web::Query<PageParams>,
	state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
	let pagination = Pagination::resolve(query.page, query.limit);
	let (users, total) = tokio::try_join!(
		state.users.list_page(pagination.limit, pagination.offset),
		state.users.count(),
	)?;
	let items: Vec<UserResponse> = users.iter().map(UserResponse::from).collect();
	Ok(HttpResponse::Ok().json(Page::new(items, &pagination, total)))
}

#[utoipa::path(tag = "users", responses((status = 201, description = "User created", body = UserResponse)))]
#[post("/users")]
pub async fn create(
	_admin: AdminUser,
	state: web::Data<AppState>,
	body: web::Json<CreateUserRequest>,
) -> Result<HttpResponse, ApiError> {
	let role = Role::from_str(&body.role)?;
	let user = state
		.users
		.create(&body.username, &body.password, body.email.clone(), role)
		.await?;
	Ok(HttpResponse::Created().json(UserResponse::from(&user)))
}

#[utoipa::path(tag = "users", responses((status = 200, description = "User updated", body = UserResponse)))]
#[patch("/users/{id}")]
pub async fn update(
	_admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
	body: web::Json<UpdateUserRequest>,
) -> Result<HttpResponse, ApiError> {
	let body = body.into_inner();
	let role = body.role.as_deref().map(Role::from_str).transpose()?;
	let user = state
		.users
		.admin_update(
			&id.into_inner(),
			body.enabled,
			role,
			body.display_name.map(Some),
		)
		.await?;
	Ok(HttpResponse::Ok().json(UserResponse::from(&user)))
}

#[utoipa::path(tag = "users", responses((status = 200, description = "Password reset", body = UserResponse), (status = 400, description = "Invalid password")))]
#[post("/users/{id}/password")]
pub async fn set_password(
	_admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
	body: web::Json<SetPasswordRequest>,
) -> Result<HttpResponse, ApiError> {
	let user = state
		.users
		.set_password(&id.into_inner(), &body.password)
		.await?;
	Ok(HttpResponse::Ok().json(UserResponse::from(&user)))
}

#[utoipa::path(tag = "users", responses((status = 204, description = "User deleted")))]
#[delete("/users/{id}")]
pub async fn delete(
	_admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	state.users.delete(&id).await?;
	Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(tag = "users", responses((status = 200, body = UserSettingsResponse)))]
#[get("/users/{id}/settings")]
pub async fn get_settings(
	_admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let id = id.into_inner();
	let settings = state
		.users
		.get_settings(&id)
		.await?
		.unwrap_or_else(|| UserSettings::default_for(&id));
	Ok(HttpResponse::Ok().json(UserSettingsResponse::from(&settings)))
}

#[utoipa::path(tag = "users", responses((status = 200, description = "Settings updated", body = UserSettingsResponse)))]
#[put("/users/{id}/settings")]
pub async fn set_settings(
	_admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
	body: web::Json<SetUserSettingsRequest>,
) -> Result<HttpResponse, ApiError> {
	let settings = state
		.users
		.set_settings(&id, body.auto_approve, body.request_quota, body.quota_days)
		.await?;
	Ok(HttpResponse::Ok().json(UserSettingsResponse::from(&settings)))
}
