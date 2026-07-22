use super::*;

pub(crate) struct NoopMailer;

#[async_trait]
impl Mailer for NoopMailer {
	async fn send(&self, _to: &str, _subject: &str, _html: &str) -> CoreResult<()> {
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
	async fn send(&self, _to: &str, _subject: &str, _html: &str) -> CoreResult<()> {
		self.sent.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
		Ok(())
	}
}
