use std::sync::Arc;

use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::consts::settings::BASE_URL;
use lunu_core::helpers::html::escape;
use lunu_core::models::NotificationEvent;
use lunu_core::repo::UserRepo;
use lunu_core::services::SettingsService;
use lunu_core::traits::{Mailer, Notifier};
use lunu_i18n::LanguageIdentifier;

use crate::optional_setting;

pub struct EmailNotifier {
	mailer: Arc<dyn Mailer>,
	users: Arc<dyn UserRepo>,
	settings: Arc<SettingsService>,
}

impl EmailNotifier {
	pub fn new(
		mailer: Arc<dyn Mailer>,
		users: Arc<dyn UserRepo>,
		settings: Arc<SettingsService>,
	) -> Self {
		Self {
			mailer,
			users,
			settings,
		}
	}

	async fn body(
		&self,
		locale: &LanguageIdentifier,
		summary: &str,
		event: &NotificationEvent,
	) -> Result<String> {
		let mut html = format!("<p>{}</p><p>{}</p>", escape(summary), escape(&event.title));
		if let Some(base) = optional_setting(&self.settings, BASE_URL).await? {
			let label = lunu_i18n::t(locale, "email-view-request");
			html.push_str(&format!(
				"<p><a href=\"{}/requests/{}\">{}</a></p>",
				escape(base.trim_end_matches('/')),
				escape(&event.request_id),
				escape(&label)
			));
		}
		Ok(html)
	}
}

#[async_trait]
impl Notifier for EmailNotifier {
	fn id(&self) -> &'static str {
		"email"
	}

	async fn deliver(&self, event: &NotificationEvent) -> Result<()> {
		let Some(user) = self.users.find_by_id(&event.user_id).await? else {
			return Ok(());
		};
		let Some(to) = user
			.email
			.as_deref()
			.map(str::trim)
			.filter(|email| !email.is_empty())
		else {
			return Ok(());
		};

		let locale = lunu_i18n::negotiate(None, user.locale.as_deref());
		let summary = lunu_i18n::t(&locale, &format!("notification-{}", event.kind.as_str()));
		let subject = format!("{summary}: {}", event.title);
		let html = self.body(&locale, &summary, event).await?;
		self.mailer.send(to, &subject, &html).await
	}
}
