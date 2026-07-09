mod health;

use actix_web::web;

pub fn routes(cfg: &mut web::ServiceConfig) {
	cfg.route("/health", web::get().to(health::health));
}
