use actix_web::{HttpResponse, get, web};
use lunu_core::consts::library::SETTING_LIBRARY_DIR;
use lunu_core::consts::settings::{PROWLARR_URL, QBITTORRENT_URL};
use lunu_core::models::{IssueStatus, JobStatus, RequestStatus};
use serde::Serialize;

use crate::error::ApiError;
use crate::extract::AdminUser;
use crate::state::AppState;

#[derive(Serialize, utoipa::ToSchema)]
pub struct Configured {
	prowlarr: bool,
	qbittorrent: bool,
	library: bool,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct Overview {
	requests_total: i64,
	requests_pending: i64,
	requests_failed: i64,
	requests_available: i64,
	downloads_total: i64,
	jobs_pending: i64,
	jobs_failed: i64,
	issues_open: i64,
	media: i64,
	users: i64,
	configured: Configured,
}

async fn is_set(state: &AppState, key: &str) -> lunu_core::Result<bool> {
	Ok(state
		.settings
		.get(key)
		.await?
		.map(|value| !value.trim().is_empty())
		.unwrap_or(false))
}

#[utoipa::path(tag = "system", responses((status = 200, description = "Aggregate system overview", body = Overview)))]
#[get("/overview")]
pub async fn overview(
	admin: AdminUser,
	state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
	let user = &admin.0;
	let (
		requests_total,
		requests_pending,
		requests_failed,
		requests_available,
		downloads_total,
		jobs_pending,
		jobs_failed,
		issues_open,
		media,
		users,
		prowlarr,
		qbittorrent,
		library,
	) = tokio::try_join!(
		state.requests.count(user, None),
		state.requests.count(user, Some(RequestStatus::Pending)),
		state.requests.count(user, Some(RequestStatus::Failed)),
		state.requests.count(user, Some(RequestStatus::Available)),
		state.grabs.count(),
		state.jobs.count(Some(JobStatus::Pending.as_str())),
		state.jobs.count(Some(JobStatus::Failed.as_str())),
		state.issues.count(Some(IssueStatus::Open)),
		state.media.count(),
		state.users.count(),
		is_set(&state, PROWLARR_URL),
		is_set(&state, QBITTORRENT_URL),
		is_set(&state, SETTING_LIBRARY_DIR),
	)?;
	let overview = Overview {
		requests_total,
		requests_pending,
		requests_failed,
		requests_available,
		downloads_total,
		jobs_pending,
		jobs_failed,
		issues_open,
		media,
		users,
		configured: Configured {
			prowlarr,
			qbittorrent,
			library,
		},
	};
	Ok(HttpResponse::Ok().json(overview))
}
