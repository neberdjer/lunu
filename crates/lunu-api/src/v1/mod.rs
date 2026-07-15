mod activity;
mod api_keys;
mod auth;
mod downloads;
mod health;
mod invites;
mod issues;
mod jobs;
mod library;
mod metadata;
mod notifications;
mod quality_profiles;
mod releases;
mod requests;
mod schedules;
mod settings;
mod setup;
mod system;
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
		.service(auth::forgot_password)
		.service(auth::reset_password)
		.service(auth::verify_email)
		.service(auth::resend_verification)
		.service(auth::sessions)
		.service(auth::revoke_session)
		.service(metadata::search)
		.service(metadata::series_search)
		.service(metadata::series_books)
		.service(metadata::series_request)
		.service(library::list)
		.service(library::sync)
		.service(library::match_media)
		.service(schedules::list)
		.service(schedules::configure)
		.service(schedules::run_now)
		.service(metadata::book_detail)
		.service(metadata::chapters)
		.service(metadata::similar)
		.service(metadata::author_books)
		.service(users::list)
		.service(users::get)
		.service(users::create)
		.service(users::update)
		.service(users::set_password)
		.service(users::delete)
		.service(users::get_settings)
		.service(users::set_settings)
		.service(requests::list)
		.service(requests::create)
		.service(requests::create_manual)
		.service(releases::search)
		.service(requests::bulk_create)
		.service(requests::bulk_approve)
		.service(requests::bulk_decline)
		.service(requests::get)
		.service(requests::delete)
		.service(requests::activity)
		.service(requests::request_download)
		.service(requests::cancel_download)
		.service(requests::retry)
		.service(requests::blocklist)
		.service(requests::blocklist_list)
		.service(requests::blocklist_remove)
		.service(requests::approve)
		.service(requests::decline)
		.service(requests::releases)
		.service(requests::grab)
		.service(downloads::list)
		.service(activity::list)
		.service(system::overview)
		.service(jobs::list)
		.service(jobs::retry)
		.service(jobs::cancel)
		.service(notifications::list)
		.service(notifications::unread_count)
		.service(notifications::mark_read)
		.service(notifications::mark_all_read)
		.service(issues::open)
		.service(issues::for_request)
		.service(issues::list)
		.service(issues::resolve)
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
