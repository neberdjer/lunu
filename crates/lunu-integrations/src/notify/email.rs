use std::sync::Arc;

use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::crypto::Encryptor;
use lunu_core::models::NotificationEvent;
use lunu_core::repo::UserRepo;
use lunu_core::services::{SettingsService, resolve_recipients};
use lunu_core::traits::{Mailer, Notifier};

pub struct EmailNotifier {
	mailer: Arc<dyn Mailer>,
	users: Arc<dyn UserRepo>,
	settings: Arc<SettingsService>,
	unsubscribe: Encryptor,
}

impl EmailNotifier {
	pub fn new(
		mailer: Arc<dyn Mailer>,
		users: Arc<dyn UserRepo>,
		settings: Arc<SettingsService>,
		unsubscribe: Encryptor,
	) -> Self {
		Self {
			mailer,
			users,
			settings,
			unsubscribe,
		}
	}

	async fn request_link(&self, event: &NotificationEvent) -> Result<Option<String>> {
		self.settings
			.app_link(&format!("requests/{}", event.request_id))
			.await
	}

	async fn unsubscribe_link(&self, user_id: &str) -> Result<Option<String>> {
		let token = self.unsubscribe.encrypt_token(user_id)?;
		self.settings
			.app_link(&format!("api/v1/unsubscribe/{token}"))
			.await
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
		if !user.notify_email {
			return Ok(());
		}
		let Some(to) = user
			.email
			.as_deref()
			.map(str::trim)
			.filter(|email| !email.is_empty())
		else {
			return Ok(());
		};

		let locale = lunu_i18n::negotiate(None, user.locale.as_deref());
		let rendered =
			lunu_core::email::notification(&locale, event.kind.as_str(), &event.title, link);
		let unsubscribe = self.unsubscribe_link(&user.id).await?;
		self.mailer
			.send_bulk(to, &rendered, unsubscribe.as_deref())
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
