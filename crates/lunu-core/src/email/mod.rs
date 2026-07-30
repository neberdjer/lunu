use askama::Template;
use lunu_i18n::LanguageIdentifier;

#[cfg(test)]
mod tests;

pub struct RenderedEmail {
	pub subject: String,
	pub html: String,
	pub text: String,
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
	intro: &'a str,
	device: &'a str,
	warning: &'a str,
}

#[derive(Template)]
#[template(path = "email/password_reset.html")]
struct PasswordResetBody<'a> {
	intro: &'a str,
	code: &'a str,
	expiry: &'a str,
}

#[derive(Template)]
#[template(path = "email/welcome.html")]
struct WelcomeBody<'a> {
	intro: &'a str,
}

#[derive(Template)]
#[template(path = "email/verification.html")]
struct VerificationBody<'a> {
	intro: &'a str,
	code: &'a str,
	expiry: &'a str,
}

#[derive(Template)]
#[template(path = "email/invite.html")]
struct InviteBody<'a> {
	intro: &'a str,
	code: &'a str,
	link: Option<&'a str>,
	accept_label: String,
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

fn wrap_text(locale: &LanguageIdentifier, lines: &[&str]) -> String {
	let mut out = vec![lunu_i18n::t(locale, "app-name")];
	out.extend(
		lines
			.iter()
			.filter(|line| !line.is_empty())
			.map(|line| line.to_string()),
	);
	out.push(lunu_i18n::t(locale, "email-footer"));
	out.join("\n\n")
}

pub fn new_device(locale: &LanguageIdentifier, device: &str) -> RenderedEmail {
	let intro = lunu_i18n::t(locale, "email-new-device-intro");
	let warning = lunu_i18n::t(locale, "email-new-device-warning");
	let body = NewDeviceBody {
		intro: &intro,
		device,
		warning: &warning,
	}
	.render()
	.expect("new device email template renders");

	RenderedEmail {
		subject: lunu_i18n::t(locale, "email-new-device-subject"),
		text: wrap_text(locale, &[&intro, device, &warning]),
		html: wrap(locale, &body),
	}
}

pub fn password_reset(locale: &LanguageIdentifier, code: &str, minutes: i64) -> RenderedEmail {
	let intro = lunu_i18n::t(locale, "email-password-reset-intro");
	let expiry = lunu_i18n::t_vars(
		locale,
		"email-password-reset-expiry",
		&[("minutes", &minutes.to_string())],
	);
	let body = PasswordResetBody {
		intro: &intro,
		code,
		expiry: &expiry,
	}
	.render()
	.expect("password reset email template renders");

	RenderedEmail {
		subject: lunu_i18n::t(locale, "email-password-reset-subject"),
		text: wrap_text(locale, &[&intro, code, &expiry]),
		html: wrap(locale, &body),
	}
}

pub fn welcome(locale: &LanguageIdentifier, username: &str) -> RenderedEmail {
	let intro = lunu_i18n::t_vars(locale, "email-welcome-intro", &[("name", username)]);
	let body = WelcomeBody { intro: &intro }
		.render()
		.expect("welcome email template renders");

	RenderedEmail {
		subject: lunu_i18n::t(locale, "email-welcome-subject"),
		text: wrap_text(locale, &[&intro]),
		html: wrap(locale, &body),
	}
}

pub fn verification(locale: &LanguageIdentifier, code: &str, minutes: i64) -> RenderedEmail {
	let intro = lunu_i18n::t(locale, "email-verification-intro");
	let expiry = lunu_i18n::t_vars(
		locale,
		"email-verification-expiry",
		&[("minutes", &minutes.to_string())],
	);
	let body = VerificationBody {
		intro: &intro,
		code,
		expiry: &expiry,
	}
	.render()
	.expect("verification email template renders");

	RenderedEmail {
		subject: lunu_i18n::t(locale, "email-verification-subject"),
		text: wrap_text(locale, &[&intro, code, &expiry]),
		html: wrap(locale, &body),
	}
}

pub fn mfa_code(locale: &LanguageIdentifier, code: &str, minutes: i64) -> RenderedEmail {
	let intro = lunu_i18n::t(locale, "email-mfa-code-intro");
	let expiry = lunu_i18n::t_vars(
		locale,
		"email-mfa-code-expiry",
		&[("minutes", &minutes.to_string())],
	);
	let body = VerificationBody {
		intro: &intro,
		code,
		expiry: &expiry,
	}
	.render()
	.expect("mfa code email template renders");

	RenderedEmail {
		subject: lunu_i18n::t(locale, "email-mfa-code-subject"),
		text: wrap_text(locale, &[&intro, code, &expiry]),
		html: wrap(locale, &body),
	}
}

pub fn invite(locale: &LanguageIdentifier, code: &str, link: Option<&str>) -> RenderedEmail {
	let intro = lunu_i18n::t(locale, "email-invite-intro");
	let body = InviteBody {
		intro: &intro,
		code,
		link,
		accept_label: lunu_i18n::t(locale, "email-invite-accept"),
	}
	.render()
	.expect("invite email template renders");

	RenderedEmail {
		subject: lunu_i18n::t(locale, "email-invite-subject"),
		text: wrap_text(locale, &[&intro, code, link.unwrap_or_default()]),
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
		text: wrap_text(locale, &[summary, title, link.unwrap_or_default()]),
		html: wrap(locale, &body),
	}
}
