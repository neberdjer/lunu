use std::sync::Arc;

use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::consts::settings::BASE_URL;
use lunu_core::models::NotificationEvent;
use lunu_core::repo::UserRepo;
use lunu_core::services::SettingsService;
use lunu_core::traits::{Mailer, Notifier};

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

	async fn request_link(&self, event: &NotificationEvent) -> Result<Option<String>> {
		let Some(base) = optional_setting(&self.settings, BASE_URL).await? else {
			return Ok(None);
		};
		Ok(Some(format!(
			"{}/requests/{}",
			base.trim_end_matches('/'),
			event.request_id
		)))
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
		let link = self.request_link(event).await?;
		let rendered =
			lunu_core::email::notification(&locale, &summary, &event.title, link.as_deref());
		self.mailer
			.send(to, &rendered.subject, &rendered.html)
			.await
	}
}
