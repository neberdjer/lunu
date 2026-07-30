use super::*;

fn locale() -> LanguageIdentifier {
	lunu_i18n::default_locale()
}

#[test]
fn every_email_resolves_its_catalog_keys() {
	let l = &locale();
	let rendered = [
		new_device(l, "Firefox"),
		password_reset(l, "CODE", 15),
		welcome(l, "Alice"),
		verification(l, "CODE", 15),
		mfa_code(l, "CODE", 15),
		invite(l, "CODE", Some("https://lunu.example/invite/CODE")),
		notification(
			l,
			"Approved",
			"Dune",
			Some("https://lunu.example/requests/1"),
		),
	];
	for email in rendered {
		for surface in [&email.subject, &email.html, &email.text] {
			assert!(
				!surface.contains("email-"),
				"unresolved email i18n key rendered: a key is missing from the catalog"
			);
		}
	}
}

#[test]
fn a_plaintext_alternative_carries_the_essentials() {
	let email = password_reset(&locale(), "ZY99", 15);
	assert!(
		email.text.contains("ZY99"),
		"the code reaches the text part"
	);
	assert!(email.text.contains("Lunu"), "the shell wraps the text part");
	assert!(!email.text.contains('<'), "the text part carries no markup");
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
fn invite_shows_the_code_and_optional_accept_link() {
	let with = invite(
		&locale(),
		"ABC123",
		Some("https://lunu.example/invite/ABC123"),
	);
	let without = invite(&locale(), "ABC123", None);
	assert!(with.html.contains("ABC123"));
	assert!(with.html.contains("https://lunu.example/invite/ABC123"));
	assert!(!without.html.contains("<a href"));
	assert_eq!(with.subject, "You are invited to Lunu");
}

#[test]
fn notification_omits_link_when_absent() {
	let with = notification(&locale(), "Approved", "Dune", Some("https://x/requests/1"));
	let without = notification(&locale(), "Approved", "Dune", None);
	assert!(with.html.contains("https://x/requests/1"));
	assert!(!without.html.contains("<a href"));
	assert_eq!(with.subject, "Approved: Dune");
}
