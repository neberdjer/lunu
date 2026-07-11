use std::str::FromStr;

use actix_web::{HttpResponse, web};
use lunu_core::models::{Role, UserSettings};
use serde::Deserialize;

use crate::dto::{UserResponse, UserSettingsResponse};
use crate::error::ApiError;
use crate::extract::AdminUser;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateUserRequest {
	username: String,
	password: String,
	email: Option<String>,
	role: String,
}

#[derive(Deserialize)]
pub struct UpdateUserRequest {
	enabled: bool,
}

#[derive(Deserialize)]
pub struct SetUserSettingsRequest {
	auto_approve: bool,
	request_quota: Option<i64>,
	quota_days: Option<i64>,
}

pub async fn list(_admin: AdminUser, state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
	let users = state.users.list().await?;
	let response: Vec<UserResponse> = users.iter().map(UserResponse::from).collect();
	Ok(HttpResponse::Ok().json(response))
}

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

pub async fn update(
	_admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
	body: web::Json<UpdateUserRequest>,
) -> Result<HttpResponse, ApiError> {
	let user = state.users.set_enabled(&id, body.enabled).await?;
	Ok(HttpResponse::Ok().json(UserResponse::from(&user)))
}

pub async fn delete(
	_admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	state.users.delete(&id).await?;
	Ok(HttpResponse::NoContent().finish())
}

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
