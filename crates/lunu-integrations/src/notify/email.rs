use std::sync::Arc;

use async_trait::async_trait;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use lunu_core::Result;
use lunu_core::consts::settings::{
	BASE_URL, DEFAULT_SMTP_ENCRYPTION, SMTP_ENCRYPTION, SMTP_FROM, SMTP_HOST, SMTP_PASSWORD,
	SMTP_PORT, SMTP_USERNAME,
};
use lunu_core::models::NotificationEvent;
use lunu_core::repo::UserRepo;
use lunu_core::services::SettingsService;
use lunu_core::traits::Notifier;

use crate::{integration_error, optional_setting};

pub struct EmailNotifier {
	settings: Arc<SettingsService>,
	users: Arc<dyn UserRepo>,
}

impl EmailNotifier {
	pub fn new(settings: Arc<SettingsService>, users: Arc<dyn UserRepo>) -> Self {
		Self { settings, users }
	}

	async fn recipient(&self, user_id: &str) -> Result<Option<String>> {
		Ok(self
			.users
			.find_by_id(user_id)
			.await?
			.and_then(|user| user.email)
			.map(|email| email.trim().to_string())
			.filter(|email| !email.is_empty()))
	}

	async fn transport(&self, host: &str) -> Result<AsyncSmtpTransport<Tokio1Executor>> {
		let mode = optional_setting(&self.settings, SMTP_ENCRYPTION)
			.await?
			.unwrap_or_else(|| DEFAULT_SMTP_ENCRYPTION.to_string());

		let (builder, default_port) = match mode.as_str() {
			"tls" => (
				AsyncSmtpTransport::<Tokio1Executor>::relay(host).map_err(integration_error)?,
				465,
			),
			"none" => (
				AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host),
				25,
			),
			_ => (
				AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
					.map_err(integration_error)?,
				587,
			),
		};

		let port = match optional_setting(&self.settings, SMTP_PORT).await? {
			Some(value) => value.parse::<u16>().map_err(integration_error)?,
			None => default_port,
		};
		let mut builder = builder.port(port);

		if let Some(username) = optional_setting(&self.settings, SMTP_USERNAME).await? {
			let password = optional_setting(&self.settings, SMTP_PASSWORD)
				.await?
				.unwrap_or_default();
			builder = builder.credentials(Credentials::new(username, password));
		}

		Ok(builder.build())
	}

	async fn body(&self, event: &NotificationEvent) -> Result<String> {
		let mut body = format!("{}: {}", event.kind.summary(), event.title);
		if let Some(base) = optional_setting(&self.settings, BASE_URL).await? {
			body.push_str(&format!(
				"\n\n{}/requests/{}",
				base.trim_end_matches('/'),
				event.request_id
			));
		}
		Ok(body)
	}
}

#[async_trait]
impl Notifier for EmailNotifier {
	fn id(&self) -> &'static str {
		"email"
	}

	async fn deliver(&self, event: &NotificationEvent) -> Result<()> {
		let Some(host) = optional_setting(&self.settings, SMTP_HOST).await? else {
			return Ok(());
		};
		let Some(from) = optional_setting(&self.settings, SMTP_FROM).await? else {
			return Ok(());
		};
		let Some(to) = self.recipient(&event.user_id).await? else {
			return Ok(());
		};

		let message = Message::builder()
			.from(from.parse::<Mailbox>().map_err(integration_error)?)
			.to(to.parse::<Mailbox>().map_err(integration_error)?)
			.subject(event.message())
			.body(self.body(event).await?)
			.map_err(integration_error)?;

		self.transport(&host)
			.await?
			.send(message)
			.await
			.map_err(integration_error)?;
		Ok(())
	}
}
