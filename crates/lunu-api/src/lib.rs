mod cookie;
mod dto;
mod error;
mod extract;
mod hub;
mod locale;
mod pagination;
mod state;
mod v1;

pub use state::AppState;

use actix_web::web;
use lunu_core::consts::api::API_PREFIX;

pub fn routes(cfg: &mut web::ServiceConfig) {
	cfg.service(web::scope(API_PREFIX).configure(v1::routes));
}
