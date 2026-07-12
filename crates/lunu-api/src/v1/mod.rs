mod activity;
mod api_keys;
mod auth;
mod downloads;
mod health;
mod invites;
mod jobs;
mod metadata;
mod quality_profiles;
mod requests;
mod settings;
mod setup;
mod users;
mod ws;

use utoipa_actix_web::service_config::ServiceConfig;

pub fn configure(cfg: &mut ServiceConfig) {
	cfg.service(health::health)
		.service(setup::status)
		.service(setup::create)
		.service(auth::login)
		.service(auth::logout)
		.service(auth::register)
		.service(auth::me)
		.service(auth::update_me)
		.service(auth::change_password)
		.service(metadata::search)
		.service(metadata::book_detail)
		.service(metadata::chapters)
		.service(users::list)
		.service(users::create)
		.service(users::update)
		.service(users::delete)
		.service(users::get_settings)
		.service(users::set_settings)
		.service(requests::list)
		.service(requests::create)
		.service(requests::get)
		.service(requests::delete)
		.service(requests::activity)
		.service(requests::retry)
		.service(requests::blocklist)
		.service(requests::approve)
		.service(requests::decline)
		.service(requests::releases)
		.service(requests::grab)
		.service(downloads::list)
		.service(activity::list)
		.service(jobs::list)
		.service(ws::ws)
		.service(quality_profiles::list)
		.service(quality_profiles::create)
		.service(quality_profiles::get)
		.service(quality_profiles::update)
		.service(quality_profiles::delete)
		.service(api_keys::list)
		.service(api_keys::create)
		.service(api_keys::delete)
		.service(invites::list)
		.service(invites::create)
		.service(invites::delete)
		.service(settings::list)
		.service(settings::test)
		.service(settings::get)
		.service(settings::set)
		.service(settings::delete);
}
