use actix_web::{HttpRequest, HttpResponse, Responder, web};
use lunu_core::consts::api::{API_VERSION, APP_NAME};
use serde::Serialize;

use crate::locale;
use crate::state::AppState;

#[derive(Serialize)]
struct HealthResponse {
	name: &'static str,
	version: &'static str,
	api: &'static str,
	database: &'static str,
	locale: String,
}

pub async fn health(req: HttpRequest, state: web::Data<AppState>) -> impl Responder {
	let database = match lunu_db::ping(&state.db).await {
		Ok(()) => "up",
		Err(_) => "down",
	};

	let locale = locale::from_request(&req).to_string();

	HttpResponse::Ok().json(HealthResponse {
		name: APP_NAME,
		version: state.version,
		api: API_VERSION,
		database,
		locale,
	})
}
