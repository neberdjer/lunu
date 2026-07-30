use std::future::Future;
use std::path::Path;

use actix_web::{HttpResponse, get, put, web};
use chrono::{DateTime, Utc};
use lunu_core::Error;
use lunu_core::consts::library::SETTING_LIBRARY_DIR;
use lunu_core::consts::logging::{DEFAULT_LOG_LIMIT, LOG_BUFFER_CAPACITY, VALID_LOG_LEVELS};
use lunu_core::consts::merge::SETTING_MERGE_ENABLED;
use lunu_core::consts::reasons;
use lunu_core::consts::settings::{DOWNLOAD_DIR, PROWLARR_URL, QBITTORRENT_URL};
use lunu_core::models::{IssueStatus, JobStatus, RequestStatus};
use lunu_core::services::LogEntry;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::extract::AdminUser;
use crate::state::AppState;

#[derive(Serialize, utoipa::ToSchema)]
pub struct LogEntryResponse {
	at: DateTime<Utc>,
	level: String,
	target: String,
	message: String,
}

impl From<LogEntry> for LogEntryResponse {
	fn from(entry: LogEntry) -> Self {
		Self {
			at: entry.at,
			level: entry.level,
			target: entry.target,
			message: entry.message,
		}
	}
}

#[derive(Deserialize, utoipa::IntoParams)]
pub struct LogQuery {
	limit: Option<usize>,
	level: Option<String>,
}

#[derive(Serialize, Deserialize, utoipa::ToSchema)]
pub struct LogLevelBody {
	level: String,
}

fn validated_level(level: &str) -> Result<String, ApiError> {
	let level = level.trim().to_lowercase();
	if !VALID_LOG_LEVELS.contains(&level.as_str()) {
		return Err(Error::Validation(reasons::INVALID_LOG_LEVEL.to_string()).into());
	}
	Ok(level)
}

#[utoipa::path(tag = "system", params(LogQuery), responses((status = 200, description = "Newest captured log lines, redacted", body = [LogEntryResponse])))]
#[get("/logs")]
pub async fn logs(
	_admin: AdminUser,
	query: web::Query<LogQuery>,
	state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
	let level = query.level.as_deref().map(validated_level).transpose()?;
	let limit = query
		.limit
		.unwrap_or(DEFAULT_LOG_LIMIT)
		.min(LOG_BUFFER_CAPACITY);
	let entries: Vec<LogEntryResponse> = state
		.logs
		.snapshot(limit, level.as_deref())
		.into_iter()
		.map(LogEntryResponse::from)
		.collect();
	Ok(HttpResponse::Ok().json(entries))
}

#[utoipa::path(tag = "system", responses((status = 200, description = "Active log level", body = LogLevelBody)))]
#[get("/log-level")]
pub async fn log_level(
	_admin: AdminUser,
	state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
	Ok(HttpResponse::Ok().json(LogLevelBody {
		level: state.log_control.current(),
	}))
}

#[utoipa::path(tag = "system", request_body = LogLevelBody, responses((status = 200, description = "Log level changed", body = LogLevelBody)))]
#[put("/log-level")]
pub async fn set_log_level(
	_admin: AdminUser,
	state: web::Data<AppState>,
	body: web::Json<LogLevelBody>,
) -> Result<HttpResponse, ApiError> {
	let level = validated_level(&body.level)?;
	if !state.log_control.set(&level) {
		return Err(Error::Internal("the log filter refused to reload".to_string()).into());
	}
	Ok(HttpResponse::Ok().json(LogLevelBody { level }))
}

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
	Ok(setting_value(state, key).await?.is_some())
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

#[derive(Serialize, utoipa::ToSchema)]
pub struct IntegrationHealth {
	name: &'static str,
	configured: bool,
	reachable: Option<bool>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct StorageHealth {
	name: &'static str,
	path: Option<String>,
	free_bytes: Option<u64>,
	total_bytes: Option<u64>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct Readiness {
	database: &'static str,
	integrations: Vec<IntegrationHealth>,
	storage: Vec<StorageHealth>,
}

async fn setting_value(state: &AppState, key: &str) -> lunu_core::Result<Option<String>> {
	Ok(state
		.settings
		.get(key)
		.await?
		.map(|value| value.trim().to_string())
		.filter(|value| !value.is_empty()))
}

async fn probe(
	name: &'static str,
	configured: bool,
	check: impl Future<Output = lunu_core::Result<()>>,
) -> IntegrationHealth {
	let reachable = if configured {
		Some(check.await.is_ok())
	} else {
		None
	};
	IntegrationHealth {
		name,
		configured,
		reachable,
	}
}

fn storage_health(name: &'static str, path: Option<String>) -> StorageHealth {
	let (free_bytes, total_bytes) = path
		.as_deref()
		.and_then(|dir| lunu_integrations::storage::disk_usage(Path::new(dir)))
		.map(|usage| (usage.free_bytes, usage.total_bytes))
		.unzip();
	StorageHealth {
		name,
		path,
		free_bytes,
		total_bytes,
	}
}

#[utoipa::path(tag = "system", responses((status = 200, description = "Dependency reachability and storage headroom", body = Readiness)))]
#[get("/readiness")]
pub async fn readiness(
	_admin: AdminUser,
	state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
	let prowlarr_configured = is_set(&state, PROWLARR_URL).await?;
	let merge_enabled = state
		.settings
		.toggle(SETTING_MERGE_ENABLED)
		.await
		.unwrap_or(false);
	let library_dir = setting_value(&state, SETTING_LIBRARY_DIR).await?;
	let download_dir = setting_value(&state, DOWNLOAD_DIR).await?;

	let mut client_probes = Vec::new();
	for client in state.download_clients.iter() {
		let configured = client.is_configured().await?;
		client_probes.push(probe(client.id(), configured, client.test_connection()));
	}

	let (database, prowlarr, ffmpeg, clients) = tokio::join!(
		lunu_db::ping(&state.db),
		probe(
			"prowlarr",
			prowlarr_configured,
			state.releases.test_indexer()
		),
		probe("ffmpeg", merge_enabled, state.merges.test()),
		futures_util::future::join_all(client_probes),
	);

	let mut integrations = Vec::with_capacity(clients.len() + 2);
	integrations.push(prowlarr);
	integrations.extend(clients);
	integrations.push(ffmpeg);

	let readiness = Readiness {
		database: if database.is_ok() { "up" } else { "down" },
		integrations,
		storage: vec![
			storage_health("library", library_dir),
			storage_health("downloads", download_dir),
		],
	};
	Ok(HttpResponse::Ok().json(readiness))
}
