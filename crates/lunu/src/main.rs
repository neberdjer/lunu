use std::process::ExitCode;
use std::sync::{Arc, OnceLock};

use actix_web::{App, HttpResponse, HttpServer, web};
use lunu_api::{AppState, LogControl};
use lunu_config::BootstrapConfig;
use lunu_core::consts::logging::LOG_BUFFER_CAPACITY;
use lunu_core::services::LogBuffer;
use lunu_jobs::{PipelineHandler, SchedulerPool, WorkerConfig, WorkerPool};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer, reload};
use utoipa::OpenApi;
use utoipa_actix_web::{AppExt, scope};

mod log;

const DEFAULT_LOG_LEVEL: &str = "info";
const QUIET_TARGETS: &str = "actix_server=warn";

fn load_dotenv() {
	if let Err(error) = dotenvy::dotenv()
		&& !error.not_found()
	{
		eprintln!("could not read .env file: {error}");
	}
}

fn init_tracing() -> (Arc<LogBuffer>, Arc<LogControl>) {
	let initial =
		std::env::var(EnvFilter::DEFAULT_ENV).unwrap_or_else(|_| DEFAULT_LOG_LEVEL.to_string());
	let filter = EnvFilter::try_from_default_env()
		.unwrap_or_else(|_| EnvFilter::new(format!("{DEFAULT_LOG_LEVEL},{QUIET_TARGETS}")));
	let (filter, handle) = reload::Layer::new(filter);
	let buffer = Arc::new(LogBuffer::new(LOG_BUFFER_CAPACITY));

	let json_logs = std::env::var(lunu_config::ENV_LOG_FORMAT)
		.map(|value| value.trim().eq_ignore_ascii_case("json"))
		.unwrap_or(false);
	let fmt_layer = if json_logs {
		tracing_subscriber::fmt::layer().json().boxed()
	} else {
		tracing_subscriber::fmt::layer().boxed()
	};

	tracing_subscriber::registry()
		.with(filter)
		.with(fmt_layer)
		.with(log::BufferLayer::new(buffer.clone()))
		.init();

	let control = Arc::new(LogControl::new(
		&initial,
		Box::new(move |level| {
			handle
				.reload(EnvFilter::new(format!("{level},{QUIET_TARGETS}")))
				.is_ok()
		}),
	));
	(buffer, control)
}

#[actix_web::main]
async fn main() -> ExitCode {
	load_dotenv();
	let (log_buffer, log_control) = init_tracing();

	let config = match BootstrapConfig::from_env() {
		Ok(config) => config,
		Err(error) => {
			eprintln!("{error}");
			return ExitCode::FAILURE;
		}
	};

	let db = match lunu_db::connect(&config.database_url).await {
		Ok(db) => db,
		Err(error) => {
			tracing::error!(%error, "failed to connect to database");
			return ExitCode::FAILURE;
		}
	};

	if let Err(error) = lunu_db::run_migrations(&db).await {
		tracing::error!(%error, "failed to run migrations");
		return ExitCode::FAILURE;
	}

	let bind = config.bind.clone();
	let workers = config.workers;

	let state = match AppState::build(
		db,
		config,
		env!("CARGO_PKG_VERSION"),
		log_buffer,
		log_control,
	) {
		Ok(state) => state,
		Err(error) => {
			tracing::error!(%error, "failed to build application state");
			return ExitCode::FAILURE;
		}
	};

	let handler = Arc::new(PipelineHandler::new(
		state.grabs.clone(),
		state.monitor.clone(),
		state.imports.clone(),
		state.merges.clone(),
		state.notifications.clone(),
		state.requests.clone(),
		state.library.clone(),
		state.auth.clone(),
		state.jobs.clone(),
		state.metadata.clone(),
		state.activity.clone(),
		state.inbox.clone(),
	));
	let worker_config = WorkerConfig::default();
	let job_runtime = match lunu_jobs::job_runtime(worker_config.workers) {
		Ok(handle) => handle,
		Err(error) => {
			tracing::error!(%error, "failed to start the job runtime");
			return ExitCode::FAILURE;
		}
	};
	WorkerPool::new(state.jobs.repo(), handler, worker_config).start(job_runtime);

	if let Err(error) = state.scheduler.ensure_defaults().await {
		tracing::error!(%error, "failed to seed default schedules");
		return ExitCode::FAILURE;
	}
	SchedulerPool::new(state.scheduler.clone()).start(job_runtime);

	let state = web::Data::new(state);
	let hsts = state.config.secure_cookies;
	let shutdown_timeout = state.config.shutdown_timeout_secs;
	let api_scope = format!("{}{}", state.config.url_base, lunu_api::API_PREFIX);
	let docs_path = format!("{}/api-docs/openapi.json", state.config.url_base);

	tracing::info!(%bind, workers, hsts, url_base = %state.config.url_base, "starting lunu");

	let server = HttpServer::new(move || {
		let (app, api) = App::new()
			.into_utoipa_app()
			.openapi(lunu_api::ApiDoc::openapi())
			.map(|app| {
				app.wrap(actix_web::middleware::from_fn(lunu_api::normalize_errors))
					.wrap(actix_web::middleware::from_fn(move |req, next| {
						lunu_api::security_headers(req, next, hsts)
					}))
					.app_data(web::JsonConfig::default().limit(lunu_api::MAX_JSON_BODY_BYTES))
					.app_data(state.clone())
			})
			.service(scope(api_scope.as_str()).configure(lunu_api::configure))
			.split_for_parts();

		static SPEC: OnceLock<web::Bytes> = OnceLock::new();
		let spec = SPEC
			.get_or_init(|| web::Bytes::from(api.to_json().unwrap_or_default()))
			.clone();
		app.route(
			&docs_path,
			web::get().to(move || {
				let spec = spec.clone();
				async move {
					HttpResponse::Ok()
						.content_type("application/json")
						.body(spec)
				}
			}),
		)
	})
	.workers(workers)
	.shutdown_timeout(shutdown_timeout)
	.bind(&bind);

	let server = match server {
		Ok(server) => server,
		Err(error) => {
			tracing::error!(%error, "failed to bind address");
			return ExitCode::FAILURE;
		}
	};

	if let Err(error) = server.run().await {
		tracing::error!(%error, "server error");
		return ExitCode::FAILURE;
	}

	ExitCode::SUCCESS
}
