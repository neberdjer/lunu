use std::sync::Arc;

use async_trait::async_trait;
use lettre::message::header::{Header, HeaderName, HeaderValue};
use lettre::message::{Mailbox, MultiPart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use lunu_core::Result;
use lunu_core::consts::settings::{
	DEFAULT_SMTP_ENCRYPTION, SMTP_ENCRYPTION, SMTP_ENCRYPTION_NONE, SMTP_ENCRYPTION_TLS, SMTP_FROM,
	SMTP_HOST, SMTP_PASSWORD, SMTP_PORT, SMTP_REPLY_TO, SMTP_USERNAME,
};
use lunu_core::email::RenderedEmail;
use lunu_core::services::SettingsService;
use lunu_core::traits::Mailer;

use crate::{integration_error, optional_setting};

macro_rules! raw_header {
	($ty:ident, $name:literal) => {
		#[derive(Clone)]
		struct $ty(String);

		impl Header for $ty {
			fn name() -> HeaderName {
				HeaderName::new_from_ascii_str($name)
			}

			fn parse(
				value: &str,
			) -> std::result::Result<Self, Box<dyn std::error::Error + Send + Sync>> {
				Ok(Self(value.to_owned()))
			}

			fn display(&self) -> HeaderValue {
				HeaderValue::new(Self::name(), self.0.clone())
			}
		}
	};
}

raw_header!(ListUnsubscribe, "List-Unsubscribe");
raw_header!(ListUnsubscribePost, "List-Unsubscribe-Post");

struct Unsubscribe {
	value: String,
	one_click: bool,
}

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

	async fn configured_endpoint(&self) -> Result<Option<(String, String)>> {
		let (Some(host), Some(from)) = (
			optional_setting(&self.settings, SMTP_HOST).await?,
			optional_setting(&self.settings, SMTP_FROM).await?,
		) else {
			return Ok(None);
		};
		Ok(Some((host, from)))
	}

	async fn deliver(
		&self,
		to: &str,
		email: &RenderedEmail,
		bulk: bool,
		one_click_url: Option<&str>,
	) -> Result<()> {
		let Some((host, from)) = self.configured_endpoint().await? else {
			return Ok(());
		};
		let reply_to = optional_setting(&self.settings, SMTP_REPLY_TO).await?;
		let unsubscribe = bulk.then(|| {
			let mailto = format!("<mailto:{from}?subject=unsubscribe>");
			match one_click_url {
				Some(url) => Unsubscribe {
					value: format!("<{url}>, {mailto}"),
					one_click: true,
				},
				None => Unsubscribe {
					value: mailto,
					one_click: false,
				},
			}
		});

		let message = build_message(&from, to, email, reply_to.as_deref(), unsubscribe.as_ref())?;

		self.transport(&host)
			.await?
			.send(message)
			.await
			.map_err(integration_error)?;
		Ok(())
	}
}

#[async_trait]
impl Mailer for SmtpMailer {
	async fn send(&self, to: &str, email: &RenderedEmail) -> Result<()> {
		self.deliver(to, email, false, None).await
	}

	async fn send_bulk(
		&self,
		to: &str,
		email: &RenderedEmail,
		unsubscribe: Option<&str>,
	) -> Result<()> {
		self.deliver(to, email, true, unsubscribe).await
	}

	async fn is_configured(&self) -> Result<bool> {
		Ok(self.configured_endpoint().await?.is_some())
	}

	async fn test_connection(&self) -> Result<()> {
		let Some((host, _from)) = self.configured_endpoint().await? else {
			return Err(lunu_core::Error::Validation(
				lunu_core::consts::reasons::SMTP_NOT_CONFIGURED.to_string(),
			));
		};
		self.transport(&host)
			.await?
			.test_connection()
			.await
			.map_err(integration_error)?
			.then_some(())
			.ok_or_else(|| integration_error("smtp server did not accept the connection"))
	}
}

fn build_message(
	from: &str,
	to: &str,
	email: &RenderedEmail,
	reply_to: Option<&str>,
	unsubscribe: Option<&Unsubscribe>,
) -> Result<Message> {
	let mut builder = Message::builder()
		.from(from.parse::<Mailbox>().map_err(integration_error)?)
		.to(to.parse::<Mailbox>().map_err(integration_error)?)
		.subject(&email.subject);

	if let Some(mailbox) = reply_to.and_then(|value| value.parse::<Mailbox>().ok()) {
		builder = builder.reply_to(mailbox);
	}
	if let Some(unsubscribe) = unsubscribe {
		builder = builder.header(ListUnsubscribe(unsubscribe.value.clone()));
		if unsubscribe.one_click {
			builder = builder.header(ListUnsubscribePost(
				"List-Unsubscribe=One-Click".to_string(),
			));
		}
	}

	builder
		.multipart(MultiPart::alternative_plain_html(
			email.text.clone(),
			email.html.clone(),
		))
		.map_err(integration_error)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn rendered(message: &Message) -> String {
		String::from_utf8(message.formatted()).unwrap()
	}

	fn email() -> RenderedEmail {
		RenderedEmail {
			subject: "Approved: Dune".to_string(),
			html: "<p>ready</p>".to_string(),
			text: "ready".to_string(),
		}
	}

	#[test]
	fn a_one_click_notification_carries_both_unsubscribe_headers_reply_to_and_a_text_part() {
		let message = build_message(
			"Lunu <no-reply@lunu.test>",
			"user@example.com",
			&email(),
			Some("support@lunu.test"),
			Some(&Unsubscribe {
				value:
					"<https://lunu.test/unsub/TOK>, <mailto:no-reply@lunu.test?subject=unsubscribe>"
						.to_string(),
				one_click: true,
			}),
		)
		.unwrap();
		let wire = rendered(&message);
		assert!(wire.contains("List-Unsubscribe: <https://lunu.test/unsub/TOK>"));
		assert!(wire.contains("List-Unsubscribe-Post: List-Unsubscribe=One-Click"));
		assert!(wire.contains("Reply-To: support@lunu.test"));
		assert!(wire.contains("multipart/alternative"));
		assert!(wire.contains("text/plain"));
		assert!(wire.contains("text/html"));
	}

	#[test]
	fn a_transactional_message_has_no_unsubscribe_and_tolerates_a_bad_reply_to() {
		let message = build_message(
			"no-reply@lunu.test",
			"user@example.com",
			&email(),
			Some("not a mailbox"),
			None,
		)
		.unwrap();
		let wire = rendered(&message);
		assert!(!wire.contains("List-Unsubscribe"));
		assert!(
			!wire.contains("Reply-To"),
			"an unparseable reply-to is skipped, not fatal"
		);
	}
}
