use std::collections::VecDeque;
use std::sync::Mutex;

use chrono::{DateTime, Utc};

use crate::consts::logging::VALID_LOG_LEVELS;
use crate::helpers::redact::redact;

#[derive(Debug, Clone)]
pub struct LogEntry {
	pub at: DateTime<Utc>,
	pub level: String,
	pub target: String,
	pub message: String,
}

fn level_rank(level: &str) -> usize {
	VALID_LOG_LEVELS
		.iter()
		.position(|known| *known == level)
		.unwrap_or(VALID_LOG_LEVELS.len())
}

pub struct LogBuffer {
	entries: Mutex<VecDeque<LogEntry>>,
	capacity: usize,
}

impl LogBuffer {
	pub fn new(capacity: usize) -> Self {
		Self {
			entries: Mutex::new(VecDeque::with_capacity(capacity)),
			capacity,
		}
	}

	pub fn record(&self, level: &str, target: &str, message: &str) {
		let entry = LogEntry {
			at: Utc::now(),
			level: level.to_ascii_lowercase(),
			target: target.to_string(),
			message: redact(message),
		};
		let mut entries = self.entries.lock().expect("log buffer lock");
		if entries.len() == self.capacity {
			entries.pop_front();
		}
		entries.push_back(entry);
	}

	pub fn snapshot(&self, limit: usize, min_level: Option<&str>) -> Vec<LogEntry> {
		let floor = min_level.map(level_rank);
		let entries = self.entries.lock().expect("log buffer lock");
		entries
			.iter()
			.rev()
			.filter(|entry| floor.is_none_or(|floor| level_rank(&entry.level) <= floor))
			.take(limit)
			.cloned()
			.collect()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn the_buffer_evicts_oldest_beyond_capacity() {
		let buffer = LogBuffer::new(2);
		buffer.record("info", "a", "first");
		buffer.record("info", "b", "second");
		buffer.record("info", "c", "third");

		let entries = buffer.snapshot(10, None);
		assert_eq!(entries.len(), 2);
		assert_eq!(entries[0].message, "third");
		assert_eq!(entries[1].message, "second");
	}

	#[test]
	fn snapshot_filters_below_the_requested_level() {
		let buffer = LogBuffer::new(10);
		buffer.record("debug", "a", "noise");
		buffer.record("warn", "b", "worth seeing");
		buffer.record("error", "c", "definitely");

		let warnings = buffer.snapshot(10, Some("warn"));
		assert_eq!(warnings.len(), 2);
		assert!(warnings.iter().all(|entry| entry.level != "debug"));
	}

	#[test]
	fn recorded_messages_are_redacted_at_capture() {
		let buffer = LogBuffer::new(10);
		buffer.record("error", "sab", "https://x/api?apikey=verysecret failed");

		let entries = buffer.snapshot(1, None);
		assert!(!entries[0].message.contains("verysecret"));
	}
}
