use std::sync::Arc;

use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::consts::settings::BASE_URL;
use lunu_core::models::NotificationEvent;
use lunu_core::repo::UserRepo;
use lunu_core::services::{SettingsService, resolve_recipients};
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

	async fn send_to(
		&self,
		user_id: &str,
		event: &NotificationEvent,
		link: Option<&str>,
	) -> Result<()> {
		let Some(user) = self.users.find_by_id(user_id).await? else {
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
		let rendered = lunu_core::email::notification(&locale, &summary, &event.title, link);
		self.mailer
			.send(to, &rendered.subject, &rendered.html)
			.await
	}
}

#[async_trait]
impl Notifier for EmailNotifier {
	fn id(&self) -> &'static str {
		"email"
	}

	async fn deliver(&self, event: &NotificationEvent) -> Result<()> {
		let link = self.request_link(event).await?;
		let mut last_error = None;
		for user_id in resolve_recipients(self.users.as_ref(), event).await? {
			if let Err(error) = self.send_to(&user_id, event, link.as_deref()).await {
				last_error = Some(error);
			}
		}
		last_error.map_or(Ok(()), Err)
	}
}
