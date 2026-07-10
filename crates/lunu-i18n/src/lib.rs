use std::borrow::Cow;
use std::collections::HashMap;

use fluent_templates::fluent_bundle::FluentValue;
use fluent_templates::{Loader, static_loader};

pub use fluent_templates::LanguageIdentifier;

static_loader! {
	static LOCALES = {
		locales: "./locales",
		fallback_language: "en-US",
	};
}

pub fn default_locale() -> LanguageIdentifier {
	"en-US"
		.parse()
		.expect("en-US is a valid language identifier")
}

pub fn available_locales() -> Vec<LanguageIdentifier> {
	LOCALES.locales().cloned().collect()
}

pub fn t(lang: &LanguageIdentifier, key: &str) -> String {
	LOCALES.lookup(lang, key)
}

pub fn t_args(
	lang: &LanguageIdentifier,
	key: &str,
	args: &HashMap<Cow<'static, str>, FluentValue>,
) -> String {
	LOCALES.lookup_with_args(lang, key, args)
}

pub fn error_message(lang: &LanguageIdentifier, code: &str, detail: Option<&str>) -> String {
	if let Some(reason) = detail
		&& let Some(message) = LOCALES.try_lookup(lang, &format!("error-{reason}"))
	{
		return message;
	}

	t(lang, &format!("error-{}", code.replace('_', "-")))
}

pub fn negotiate(accept_language: Option<&str>, user_pref: Option<&str>) -> LanguageIdentifier {
	let mut requested: Vec<LanguageIdentifier> = Vec::new();

	if let Some(pref) = user_pref
		&& let Ok(lang) = pref.parse::<LanguageIdentifier>()
	{
		requested.push(lang);
	}

	if let Some(header) = accept_language {
		requested.extend(parse_accept_language(header));
	}

	for want in &requested {
		if let Some(found) = LOCALES
			.locales()
			.find(|have| have.language == want.language)
		{
			return found.clone();
		}
	}

	default_locale()
}

fn parse_accept_language(header: &str) -> Vec<LanguageIdentifier> {
	let mut items: Vec<(f32, LanguageIdentifier)> = header
		.split(',')
		.filter_map(|part| {
			let mut segments = part.split(';');
			let tag = segments.next()?.trim();
			if tag.is_empty() || tag == "*" {
				return None;
			}
			let lang = tag.parse::<LanguageIdentifier>().ok()?;
			let quality = segments
				.find_map(|segment| segment.trim().strip_prefix("q="))
				.and_then(|value| value.parse::<f32>().ok())
				.unwrap_or(1.0);
			Some((quality, lang))
		})
		.collect();

	items.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
	items.into_iter().map(|(_, lang)| lang).collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn looks_up_app_name() {
		assert_eq!(t(&default_locale(), "app-name"), "Lunu");
	}

	#[test]
	fn negotiates_known_language_from_header() {
		let lang = negotiate(Some("en-GB,en;q=0.9"), None);
		assert_eq!(lang.language.as_str(), "en");
	}

	#[test]
	fn falls_back_to_default_for_unknown() {
		let lang = negotiate(Some("zz"), None);
		assert_eq!(lang, default_locale());
	}

	#[test]
	fn maps_error_code_to_generic_message() {
		let message = error_message(&default_locale(), "not_found", None);
		assert_eq!(message, "The requested resource was not found.");
	}

	#[test]
	fn resolves_specific_reason_key() {
		let message = error_message(&default_locale(), "conflict", Some("username-taken"));
		assert_eq!(message, "That username is already taken.");
	}

	#[test]
	fn falls_back_to_generic_when_reason_missing() {
		let message = error_message(&default_locale(), "not_found", Some("user 123"));
		assert_eq!(message, "The requested resource was not found.");
	}
}
