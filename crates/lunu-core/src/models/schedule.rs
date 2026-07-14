use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Schedule {
	pub kind: String,
	pub interval_secs: i64,
	pub enabled: bool,
	pub next_run_at: DateTime<Utc>,
	pub last_run_at: Option<DateTime<Utc>>,
	pub updated_at: DateTime<Utc>,
}
