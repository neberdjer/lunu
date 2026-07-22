use std::sync::Arc;

use async_trait::async_trait;
use lettre::message::{Mailbox, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use lunu_core::Result;
use lunu_core::consts::settings::{
	DEFAULT_SMTP_ENCRYPTION, SMTP_ENCRYPTION, SMTP_ENCRYPTION_NONE, SMTP_ENCRYPTION_TLS, SMTP_FROM,
	SMTP_HOST, SMTP_PASSWORD, SMTP_PORT, SMTP_USERNAME,
};
use lunu_core::services::SettingsService;
use lunu_core::traits::Mailer;

use crate::{integration_error, optional_setting};

pub struct SmtpMailer {
	settings: Arc<SettingsService>,
	cached: tokio::sync::RwLock<Option<(String, AsyncSmtpTransport<Tokio1Executor>)>>,
}

impl SmtpMailer {
	pub fn new(settings: Arc<SettingsService>) -> Self {
		Self {
			settings,
			cached: tokio::sync::RwLock::new(None),
		}
	}

	async fn transport(&self, host: &str) -> Result<AsyncSmtpTransport<Tokio1Executor>> {
		let fingerprint = self.fingerprint(host).await?;
		if let Some((cached_for, transport)) = self.cached.read().await.as_ref()
			&& *cached_for == fingerprint
		{
			return Ok(transport.clone());
		}

		let built = self.build_transport(host).await?;
		*self.cached.write().await = Some((fingerprint, built.clone()));
		Ok(built)
	}

	async fn fingerprint(&self, host: &str) -> Result<String> {
		let parts = [
			Some(host.to_string()),
			optional_setting(&self.settings, SMTP_ENCRYPTION).await?,
			optional_setting(&self.settings, SMTP_PORT).await?,
			optional_setting(&self.settings, SMTP_USERNAME).await?,
			optional_setting(&self.settings, SMTP_PASSWORD).await?,
		];
		Ok(parts
			.iter()
			.map(|part| part.as_deref().unwrap_or_default())
			.collect::<Vec<_>>()
			.join("\u{1}"))
	}

	async fn build_transport(&self, host: &str) -> Result<AsyncSmtpTransport<Tokio1Executor>> {
		let mode = optional_setting(&self.settings, SMTP_ENCRYPTION)
			.await?
			.unwrap_or_else(|| DEFAULT_SMTP_ENCRYPTION.to_string());

		let (builder, default_port) = match mode.as_str() {
			SMTP_ENCRYPTION_TLS => (
				AsyncSmtpTransport::<Tokio1Executor>::relay(host).map_err(integration_error)?,
				465,
			),
			SMTP_ENCRYPTION_NONE => (
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
}

#[async_trait]
impl Mailer for SmtpMailer {
	async fn send(&self, to: &str, subject: &str, html: &str) -> Result<()> {
		let Some(host) = optional_setting(&self.settings, SMTP_HOST).await? else {
			return Ok(());
		};
		let Some(from) = optional_setting(&self.settings, SMTP_FROM).await? else {
			return Ok(());
		};

		let message = Message::builder()
			.from(from.parse::<Mailbox>().map_err(integration_error)?)
			.to(to.parse::<Mailbox>().map_err(integration_error)?)
			.subject(subject)
			.singlepart(SinglePart::html(html.to_string()))
			.map_err(integration_error)?;

		self.transport(&host)
			.await?
			.send(message)
			.await
			.map_err(integration_error)?;
		Ok(())
	}
}
