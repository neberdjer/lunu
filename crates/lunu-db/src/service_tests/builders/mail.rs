use lunu_core::email::RenderedEmail;

use super::*;

pub(crate) struct NoopMailer;

#[async_trait]
impl Mailer for NoopMailer {
	async fn send(&self, _to: &str, _email: &RenderedEmail) -> CoreResult<()> {
		Ok(())
	}
	async fn send_bulk(
		&self,
		_to: &str,
		_email: &RenderedEmail,
		_unsubscribe: Option<&str>,
	) -> CoreResult<()> {
		Ok(())
	}
	async fn is_configured(&self) -> CoreResult<bool> {
		Ok(true)
	}
	async fn test_connection(&self) -> CoreResult<()> {
		Ok(())
	}
}

#[derive(Default)]
pub(crate) struct RecordingMailer {
	sent: std::sync::atomic::AtomicUsize,
}

impl RecordingMailer {
	pub(crate) fn count(&self) -> usize {
		self.sent.load(std::sync::atomic::Ordering::Relaxed)
	}
}

#[async_trait]
impl Mailer for RecordingMailer {
	async fn send(&self, _to: &str, _email: &RenderedEmail) -> CoreResult<()> {
		self.sent.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
		Ok(())
	}
	async fn send_bulk(
		&self,
		to: &str,
		email: &RenderedEmail,
		_unsubscribe: Option<&str>,
	) -> CoreResult<()> {
		self.send(to, email).await
	}
	async fn is_configured(&self) -> CoreResult<bool> {
		Ok(true)
	}
	async fn test_connection(&self) -> CoreResult<()> {
		Ok(())
	}
}
