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
		invite(
			l,
			"CODE",
			Some("https://lunu.example/invite/CODE"),
			Some("May 1, 2026"),
		),
		notification(
			l,
			"request-approved",
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
fn invite_shows_the_code_link_and_optional_expiry() {
	let with = invite(
		&locale(),
		"ABC123",
		Some("https://lunu.example/invite/ABC123"),
		Some("May 1, 2026"),
	);
	let bare = invite(&locale(), "ABC123", None, None);
	assert!(with.html.contains("ABC123"));
	assert!(with.html.contains("https://lunu.example/invite/ABC123"));
	assert!(with.html.contains("This invitation expires"));
	assert!(with.html.contains("May 1, 2026"));
	assert!(with.text.contains("May 1, 2026"));
	assert!(!bare.html.contains("<a href"));
	assert!(
		!bare.html.contains("This invitation expires"),
		"no expiry line when the invite never expires"
	);
	assert_eq!(with.subject, "You are invited to Lunu");
}

#[test]
fn a_notification_reads_as_a_sentence_not_a_ui_label() {
	let with = notification(
		&locale(),
		"request-available",
		"Dune",
		Some("https://x/requests/1"),
	);
	let without = notification(&locale(), "request-approved", "Dune", None);
	assert!(
		with.html.contains("is now available in your library."),
		"the body is a full sentence, not a bare label"
	);
	assert!(with.html.contains("Dune"));
	assert!(with.html.contains("https://x/requests/1"));
	assert!(!without.html.contains("<a href"));
	assert_eq!(
		with.subject, "Now available: Dune",
		"the subject keeps the short scannable label"
	);
}
