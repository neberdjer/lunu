use actix_web::{HttpResponse, get, patch, post, web};
use lunu_core::Error;
use serde::Deserialize;
use serde_json::json;

use crate::dto::ScheduleResponse;
use crate::error::ApiError;
use crate::extract::AdminUser;
use crate::state::AppState;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ConfigureScheduleBody {
	enabled: bool,
	interval_secs: i64,
}

#[utoipa::path(tag = "schedules", responses((status = 200, description = "Recurring task schedules")))]
#[get("/admin/schedules")]
pub async fn list(_admin: AdminUser, state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
	let schedules = state.scheduler.list().await?;
	let items: Vec<ScheduleResponse> = schedules.iter().map(ScheduleResponse::from).collect();
	Ok(HttpResponse::Ok().json(items))
}

#[utoipa::path(tag = "schedules", responses((status = 200, description = "Schedule updated"), (status = 404, description = "Unknown schedule")))]
#[patch("/admin/schedules/{kind}")]
pub async fn configure(
	_admin: AdminUser,
	state: web::Data<AppState>,
	kind: web::Path<String>,
	body: web::Json<ConfigureScheduleBody>,
) -> Result<HttpResponse, ApiError> {
	let kind = kind.into_inner();
	let body = body.into_inner();
	let updated = state
		.scheduler
		.configure(&kind, body.enabled, body.interval_secs)
		.await?;
	if !updated {
		return Err(Error::NotFound(format!("schedule {kind}")).into());
	}
	Ok(HttpResponse::Ok().json(json!({ "status": "ok" })))
}

#[utoipa::path(tag = "schedules", responses((status = 202, description = "Due schedules run now")))]
#[post("/admin/schedules/run")]
pub async fn run_now(
	_admin: AdminUser,
	state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
	let enqueued = state.scheduler.run_due().await?;
	Ok(HttpResponse::Accepted().json(json!({ "enqueued": enqueued })))
}
