use actix_web::HttpRequest;
use actix_web::http::header::ACCEPT_LANGUAGE;
use lunu_i18n::LanguageIdentifier;

pub fn from_request(req: &HttpRequest) -> LanguageIdentifier {
	let accept_language = req
		.headers()
		.get(ACCEPT_LANGUAGE)
		.and_then(|value| value.to_str().ok());

	lunu_i18n::negotiate(accept_language, None)
}
