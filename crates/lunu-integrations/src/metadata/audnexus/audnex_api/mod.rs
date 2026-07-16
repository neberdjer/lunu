use lunu_core::Result;
use lunu_core::models::{Book, Chapters, SeriesRef};
use reqwest::StatusCode;
use serde::de::DeserializeOwned;

use super::audible_api;
use crate::http::send_with_retry;
use crate::integration_error;

mod book;
mod chapters;

use book::{AudnexusAuthor, AudnexusBook};
use chapters::AudnexusChapters;

const AUDNEXUS_BASE: &str = "https://api.audnex.us";

pub(super) async fn get_book(
	client: &reqwest::Client,
	region: &str,
	asin: &str,
) -> Result<Option<Book>> {
	let Some(body) = get_json::<AudnexusBook>(client, region, &format!("books/{asin}")).await?
	else {
		return Ok(None);
	};

	let mut book = body.into_book();
	merge_audible_series(client, region, &mut book).await;
	Ok(Some(book))
}

pub(super) async fn get_author_name(
	client: &reqwest::Client,
	region: &str,
	asin: &str,
) -> Result<Option<String>> {
	Ok(
		get_json::<AudnexusAuthor>(client, region, &format!("authors/{asin}"))
			.await?
			.map(|author| author.name),
	)
}

pub(super) async fn get_chapters(
	client: &reqwest::Client,
	region: &str,
	asin: &str,
) -> Result<Option<Chapters>> {
	Ok(
		get_json::<AudnexusChapters>(client, region, &format!("books/{asin}/chapters"))
			.await?
			.map(AudnexusChapters::into_chapters),
	)
}

async fn merge_audible_series(client: &reqwest::Client, region: &str, book: &mut Book) {
	let Some(asin) = book.asin().map(str::to_string) else {
		return;
	};
	let parents = match audible_api::series_parents(client, region, &asin).await {
		Ok(parents) => parents,
		Err(error) => {
			tracing::debug!(
				%asin,
				%error,
				"audible series lookup failed, keeping audnexus series only"
			);
			return;
		}
	};

	for parent in parents {
		let known = book
			.series
			.iter()
			.any(|entry| entry.asin.as_deref() == Some(parent.asin.as_str()));
		if known {
			continue;
		}
		let Some(name) = parent.title else {
			continue;
		};
		book.series.push(SeriesRef {
			name,
			position: parent.sequence,
			asin: Some(parent.asin),
		});
	}
}

async fn get_json<T: DeserializeOwned>(
	client: &reqwest::Client,
	region: &str,
	path: &str,
) -> Result<Option<T>> {
	let url = format!("{AUDNEXUS_BASE}/{path}");
	let response = send_with_retry(|| client.get(&url).query(&[("region", region)])).await?;

	let status = response.status();
	if status == StatusCode::NOT_FOUND || status == StatusCode::BAD_REQUEST {
		return Ok(None);
	}

	let response = response.error_for_status().map_err(integration_error)?;
	Ok(Some(response.json::<T>().await.map_err(integration_error)?))
}
