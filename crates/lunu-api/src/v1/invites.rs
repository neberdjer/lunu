use std::str::FromStr;

use actix_web::{HttpResponse, web};
use chrono::{Duration, Utc};
use lunu_core::consts::auth::DEFAULT_INVITE_MAX_USES;
use lunu_core::models::Role;
use serde::Deserialize;
use serde_json::json;

use crate::dto::InviteResponse;
use crate::error::ApiError;
use crate::extract::AdminUser;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateInviteRequest {
	role: String,
	email: Option<String>,
	max_uses: Option<i64>,
	expires_in_days: Option<i64>,
}

pub async fn list(_admin: AdminUser, state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
	let invites = state.invites.list().await?;
	let response: Vec<InviteResponse> = invites.iter().map(InviteResponse::from).collect();
	Ok(HttpResponse::Ok().json(response))
}

pub async fn create(
	admin: AdminUser,
	state: web::Data<AppState>,
	body: web::Json<CreateInviteRequest>,
) -> Result<HttpResponse, ApiError> {
	let role = Role::from_str(&body.role)?;
	let expires_at = body
		.expires_in_days
		.map(|days| Utc::now() + Duration::days(days));

	let issued = state
		.invites
		.create(
			&admin.id,
			role,
			body.email.clone(),
			body.max_uses.unwrap_or(DEFAULT_INVITE_MAX_USES),
			expires_at,
		)
		.await?;

	Ok(HttpResponse::Created().json(json!({
		"code": issued.code,
		"invite": InviteResponse::from(&issued.invite),
	})))
}

pub async fn delete(
	_admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	state.invites.delete(&id).await?;
	Ok(HttpResponse::NoContent().finish())
}
