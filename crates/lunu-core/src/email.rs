use askama::Template;
use lunu_i18n::LanguageIdentifier;

pub struct RenderedEmail {
	pub subject: String,
	pub html: String,
}

#[derive(Template)]
#[template(path = "email/base.html")]
struct Shell<'a> {
	app_name: String,
	footer: String,
	body: &'a str,
}

#[derive(Template)]
#[template(path = "email/new_device.html")]
struct NewDeviceBody<'a> {
	intro: String,
	device: &'a str,
	warning: String,
}

#[derive(Template)]
#[template(path = "email/password_reset.html")]
struct PasswordResetBody<'a> {
	intro: String,
	code: &'a str,
	expiry: String,
}

#[derive(Template)]
#[template(path = "email/welcome.html")]
struct WelcomeBody {
	intro: String,
}

#[derive(Template)]
#[template(path = "email/verification.html")]
struct VerificationBody<'a> {
	intro: String,
	code: &'a str,
	expiry: String,
}

#[derive(Template)]
#[template(path = "email/notification.html")]
struct NotificationBody<'a> {
	summary: &'a str,
	title: &'a str,
	link: Option<&'a str>,
	link_label: String,
}

fn wrap(locale: &LanguageIdentifier, body: &str) -> String {
	Shell {
		app_name: lunu_i18n::t(locale, "app-name"),
		footer: lunu_i18n::t(locale, "email-footer"),
		body,
	}
	.render()
	.expect("email shell template renders")
}

pub fn new_device(locale: &LanguageIdentifier, device: &str) -> RenderedEmail {
	let body = NewDeviceBody {
		intro: lunu_i18n::t(locale, "email-new-device-intro"),
		device,
		warning: lunu_i18n::t(locale, "email-new-device-warning"),
	}
	.render()
	.expect("new device email template renders");

	RenderedEmail {
		subject: lunu_i18n::t(locale, "email-new-device-subject"),
		html: wrap(locale, &body),
	}
}

pub fn password_reset(locale: &LanguageIdentifier, code: &str, minutes: i64) -> RenderedEmail {
	let expiry = lunu_i18n::t_vars(
		locale,
		"email-password-reset-expiry",
		&[("minutes", &minutes.to_string())],
	);
	let body = PasswordResetBody {
		intro: lunu_i18n::t(locale, "email-password-reset-intro"),
		code,
		expiry,
	}
	.render()
	.expect("password reset email template renders");

	RenderedEmail {
		subject: lunu_i18n::t(locale, "email-password-reset-subject"),
		html: wrap(locale, &body),
	}
}

pub fn welcome(locale: &LanguageIdentifier, username: &str) -> RenderedEmail {
	let body = WelcomeBody {
		intro: lunu_i18n::t_vars(locale, "email-welcome-intro", &[("name", username)]),
	}
	.render()
	.expect("welcome email template renders");

	RenderedEmail {
		subject: lunu_i18n::t(locale, "email-welcome-subject"),
		html: wrap(locale, &body),
	}
}

pub fn verification(locale: &LanguageIdentifier, code: &str, minutes: i64) -> RenderedEmail {
	let expiry = lunu_i18n::t_vars(
		locale,
		"email-verification-expiry",
		&[("minutes", &minutes.to_string())],
	);
	let body = VerificationBody {
		intro: lunu_i18n::t(locale, "email-verification-intro"),
		code,
		expiry,
	}
	.render()
	.expect("verification email template renders");

	RenderedEmail {
		subject: lunu_i18n::t(locale, "email-verification-subject"),
		html: wrap(locale, &body),
	}
}

pub fn mfa_code(locale: &LanguageIdentifier, code: &str, minutes: i64) -> RenderedEmail {
	let expiry = lunu_i18n::t_vars(
		locale,
		"email-mfa-code-expiry",
		&[("minutes", &minutes.to_string())],
	);
	let body = VerificationBody {
		intro: lunu_i18n::t(locale, "email-mfa-code-intro"),
		code,
		expiry,
	}
	.render()
	.expect("mfa code email template renders");

	RenderedEmail {
		subject: lunu_i18n::t(locale, "email-mfa-code-subject"),
		html: wrap(locale, &body),
	}
}

pub fn notification(
	locale: &LanguageIdentifier,
	summary: &str,
	title: &str,
	link: Option<&str>,
) -> RenderedEmail {
	let body = NotificationBody {
		summary,
		title,
		link,
		link_label: link
			.map(|_| lunu_i18n::t(locale, "email-view-request"))
			.unwrap_or_default(),
	}
	.render()
	.expect("notification email template renders");

	RenderedEmail {
		subject: format!("{summary}: {title}"),
		html: wrap(locale, &body),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn locale() -> LanguageIdentifier {
		lunu_i18n::default_locale()
	}

	#[test]
	fn new_device_escapes_device_and_wraps_in_shell() {
		let rendered = new_device(&locale(), "<script>Firefox");
		assert!(rendered.html.contains("Firefox"));
		assert!(!rendered.html.contains("<script>"));
		assert!(rendered.html.contains("<h1>Lunu</h1>"));
		assert_eq!(rendered.subject, "New sign-in to your Lunu account");
	}

	#[test]
	fn notification_omits_link_when_absent() {
		let with = notification(&locale(), "Approved", "Dune", Some("https://x/requests/1"));
		let without = notification(&locale(), "Approved", "Dune", None);
		assert!(with.html.contains("https://x/requests/1"));
		assert!(!without.html.contains("<a href"));
		assert_eq!(with.subject, "Approved: Dune");
	}
}
