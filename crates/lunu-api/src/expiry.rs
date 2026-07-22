use chrono::{DateTime, Duration, Utc};
use lunu_core::Error;
use lunu_core::consts::MAX_EXPIRY_DAYS;
use lunu_core::consts::reasons;

pub(crate) fn resolve(expires_in_days: Option<i64>) -> Result<Option<DateTime<Utc>>, Error> {
	let Some(days) = expires_in_days else {
		return Ok(None);
	};
	if !(1..=MAX_EXPIRY_DAYS).contains(&days) {
		return Err(Error::Validation(reasons::INVALID_EXPIRY.to_string()));
	}
	Ok(Some(Utc::now() + Duration::days(days)))
}
