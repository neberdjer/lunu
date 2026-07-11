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

use actix_web::web;

pub fn routes(cfg: &mut web::ServiceConfig) {
	cfg.route("/health", web::get().to(health::health));

	cfg.route("/setup", web::get().to(setup::status));
	cfg.route("/setup", web::post().to(setup::create));

	cfg.route("/auth/login", web::post().to(auth::login));
	cfg.route("/auth/logout", web::post().to(auth::logout));
	cfg.route("/auth/register", web::post().to(auth::register));
	cfg.route("/auth/me", web::get().to(auth::me));

	cfg.route("/search", web::get().to(metadata::search));
	cfg.route("/books/{asin}", web::get().to(metadata::book));
	cfg.route("/books/{asin}/chapters", web::get().to(metadata::chapters));

	cfg.route("/users", web::get().to(users::list));
	cfg.route("/users", web::post().to(users::create));
	cfg.route("/users/{id}", web::patch().to(users::update));
	cfg.route("/users/{id}", web::delete().to(users::delete));
	cfg.route("/users/{id}/settings", web::get().to(users::get_settings));
	cfg.route("/users/{id}/settings", web::put().to(users::set_settings));

	cfg.route("/requests", web::get().to(requests::list));
	cfg.route("/requests", web::post().to(requests::create));
	cfg.route("/requests/{id}", web::get().to(requests::get));
	cfg.route("/requests/{id}", web::delete().to(requests::delete));
	cfg.route("/requests/{id}/activity", web::get().to(requests::activity));
	cfg.route("/requests/{id}/retry", web::post().to(requests::retry));
	cfg.route(
		"/requests/{id}/blocklist",
		web::post().to(requests::blocklist),
	);
	cfg.route("/requests/{id}/approve", web::post().to(requests::approve));
	cfg.route("/requests/{id}/decline", web::post().to(requests::decline));
	cfg.route("/requests/{id}/releases", web::get().to(requests::releases));
	cfg.route("/requests/{id}/grab", web::post().to(requests::grab));

	cfg.route("/downloads", web::get().to(downloads::list));

	cfg.route("/activity", web::get().to(activity::list));
	cfg.route("/jobs", web::get().to(jobs::list));
	cfg.route("/ws", web::get().to(ws::ws));

	cfg.route("/quality-profiles", web::get().to(quality_profiles::list));
	cfg.route(
		"/quality-profiles",
		web::post().to(quality_profiles::create),
	);
	cfg.route(
		"/quality-profiles/{id}",
		web::get().to(quality_profiles::get),
	);
	cfg.route(
		"/quality-profiles/{id}",
		web::put().to(quality_profiles::update),
	);
	cfg.route(
		"/quality-profiles/{id}",
		web::delete().to(quality_profiles::delete),
	);

	cfg.route("/api-keys", web::get().to(api_keys::list));
	cfg.route("/api-keys", web::post().to(api_keys::create));
	cfg.route("/api-keys/{id}", web::delete().to(api_keys::delete));

	cfg.route("/invites", web::get().to(invites::list));
	cfg.route("/invites", web::post().to(invites::create));
	cfg.route("/invites/{id}", web::delete().to(invites::delete));

	cfg.route("/settings", web::get().to(settings::list));
	cfg.route(
		"/settings/test/{integration}",
		web::post().to(settings::test),
	);
	cfg.route("/settings/{key}", web::get().to(settings::get));
	cfg.route("/settings/{key}", web::put().to(settings::set));
	cfg.route("/settings/{key}", web::delete().to(settings::delete));
}
