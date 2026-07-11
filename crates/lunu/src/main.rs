use std::process::ExitCode;
use std::sync::Arc;

use actix_web::{App, HttpServer, web};
use lunu_api::AppState;
use lunu_config::BootstrapConfig;
use lunu_jobs::{PipelineHandler, WorkerConfig, WorkerPool};
use tracing_subscriber::EnvFilter;

const DEFAULT_LOG_FILTER: &str = "info,actix_server=warn";

fn load_dotenv() {
	if let Err(error) = dotenvy::dotenv()
		&& !error.not_found()
	{
		eprintln!("could not read .env file: {error}");
	}
}

fn init_tracing() {
	let filter =
		EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_FILTER));
	tracing_subscriber::fmt().with_env_filter(filter).init();
}

#[actix_web::main]
async fn main() -> ExitCode {
	load_dotenv();
	init_tracing();

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

	let state = match AppState::build(db, config, env!("CARGO_PKG_VERSION")) {
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
	));
	WorkerPool::new(state.jobs.repo(), handler, WorkerConfig::default()).start();

	let state = web::Data::new(state);

	tracing::info!(%bind, workers, "starting lunu");

	let server = HttpServer::new(move || {
		App::new()
			.app_data(state.clone())
			.configure(lunu_api::routes)
	})
	.workers(workers)
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
