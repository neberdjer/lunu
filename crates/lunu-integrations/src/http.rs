use std::time::Duration;

use lunu_core::Result;
use reqwest::StatusCode;
use tokio::time::sleep;

use crate::integration_error;

const MAX_RETRIES: u32 = 3;
const RETRY_BASE_MS: u64 = 500;
const RETRY_MAX_WAIT_SECS: u64 = 10;

pub(crate) async fn send_with_retry<F>(build: F) -> Result<reqwest::Response>
where
	F: Fn() -> reqwest::RequestBuilder,
{
	let mut attempt: u32 = 0;

	loop {
		match build().send().await {
			Ok(response) => {
				let status = response.status();
				let retryable = status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error();
				if retryable && attempt < MAX_RETRIES {
					let wait = retry_after(&response).unwrap_or_else(|| backoff(attempt));
					sleep(wait).await;
					attempt += 1;
					continue;
				}
				return Ok(response);
			}
			Err(error) => {
				if attempt < MAX_RETRIES && error.is_connect() {
					sleep(backoff(attempt)).await;
					attempt += 1;
					continue;
				}
				return Err(integration_error(error));
			}
		}
	}
}

fn backoff(attempt: u32) -> Duration {
	Duration::from_millis(RETRY_BASE_MS << attempt)
}

fn retry_after(response: &reqwest::Response) -> Option<Duration> {
	let seconds = response
		.headers()
		.get(reqwest::header::RETRY_AFTER)?
		.to_str()
		.ok()?
		.parse::<u64>()
		.ok()?;
	Some(Duration::from_secs(seconds.min(RETRY_MAX_WAIT_SECS)))
}
