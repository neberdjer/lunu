use std::fmt::Write;
use std::sync::Arc;

use lunu_core::services::LogBuffer;
use tracing::field::{Field, Visit};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

pub struct BufferLayer {
	buffer: Arc<LogBuffer>,
}

impl BufferLayer {
	pub fn new(buffer: Arc<LogBuffer>) -> Self {
		Self { buffer }
	}
}

#[derive(Default)]
struct EventText {
	message: String,
	fields: String,
}

impl Visit for EventText {
	fn record_str(&mut self, field: &Field, value: &str) {
		if field.name() == "message" {
			self.message = value.to_string();
		} else {
			let _ = write!(self.fields, " {}={}", field.name(), value);
		}
	}

	fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
		if field.name() == "message" {
			self.message = format!("{value:?}");
		} else {
			let _ = write!(self.fields, " {}={:?}", field.name(), value);
		}
	}
}

fn level_name(level: &tracing::Level) -> &'static str {
	match *level {
		tracing::Level::ERROR => "error",
		tracing::Level::WARN => "warn",
		tracing::Level::INFO => "info",
		tracing::Level::DEBUG => "debug",
		tracing::Level::TRACE => "trace",
	}
}

impl<S: tracing::Subscriber> Layer<S> for BufferLayer {
	fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
		let mut text = EventText::default();
		event.record(&mut text);
		text.message.push_str(&text.fields);
		self.buffer.record(
			level_name(event.metadata().level()),
			event.metadata().target(),
			&text.message,
		);
	}
}
