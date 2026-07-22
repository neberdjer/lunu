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
	send_retrying(build, true).await
}

pub(crate) async fn send_write<F>(build: F) -> Result<reqwest::Response>
where
	F: Fn() -> reqwest::RequestBuilder,
{
	send_retrying(build, false).await
}

async fn send_retrying<F>(build: F, replayable: bool) -> Result<reqwest::Response>
where
	F: Fn() -> reqwest::RequestBuilder,
{
	let mut attempt: u32 = 0;

	loop {
		match build().send().await {
			Ok(response) => {
				let status = response.status();
				let retryable = replayable
					&& (status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error());
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
	let base = RETRY_BASE_MS << attempt;
	let spread = base / 4;
	Duration::from_millis(base.saturating_sub(spread) + jitter(spread * 2 + 1))
}

fn jitter(range: u64) -> u64 {
	std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.map(|since| u64::from(since.subsec_nanos()) % range)
		.unwrap_or(0)
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

pub(crate) async fn get_json<T, F>(build: F) -> Result<T>
where
	T: serde::de::DeserializeOwned,
	F: Fn() -> reqwest::RequestBuilder,
{
	send_with_retry(build)
		.await?
		.error_for_status()
		.map_err(crate::integration_error)?
		.json()
		.await
		.map_err(crate::integration_error)
}
