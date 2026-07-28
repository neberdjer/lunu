use std::sync::Arc;

use chrono::Utc;

use crate::consts::reasons;
use crate::models::{ExternalId, Format, User, Watch};
use crate::repo::WatchRepo;
use crate::services::{MetadataService, RequestService, WorkService, new_id};
use crate::{Error, Result};

pub struct WatchService {
	watches: Arc<dyn WatchRepo>,
	metadata: Arc<MetadataService>,
	works: Arc<WorkService>,
	requests: Arc<RequestService>,
}

impl WatchService {
	pub fn new(
		watches: Arc<dyn WatchRepo>,
		metadata: Arc<MetadataService>,
		works: Arc<WorkService>,
		requests: Arc<RequestService>,
	) -> Self {
		Self {
			watches,
			metadata,
			works,
			requests,
		}
	}

	pub async fn create(&self, user: &User, id: &ExternalId) -> Result<Watch> {
		let book = self
			.metadata
			.get_book(id)
			.await?
			.ok_or_else(|| Error::Validation(reasons::INVALID_ASIN.to_string()))?;
		let work_id = self
			.works
			.for_book(&book)
			.await?
			.ok_or_else(|| Error::Validation(reasons::INVALID_ASIN.to_string()))?;

		let asin = book.asin().map(str::to_string);
		let metadata_region = Some(self.metadata.region_or_current(id.region.clone()).await?);
		let series = book.series.into_iter().next();
		let watch = Watch {
			id: new_id(),
			user_id: user.id.clone(),
			work_id,
			format: Format::Audiobook,
			asin,
			title: book.title,
			author: book.authors.into_iter().next(),
			cover_url: book.cover_url,
			series_name: series.as_ref().map(|entry| entry.name.clone()),
			series_sequence: series.and_then(|entry| entry.position),
			metadata_region,
			created_at: Utc::now(),
		};
		self.watches.create(&watch).await?;
		Ok(watch)
	}

	pub async fn list_page(&self, user_id: &str, limit: i64, offset: i64) -> Result<Vec<Watch>> {
		self.watches.list_page(user_id, limit, offset).await
	}

	pub async fn count(&self, user_id: &str) -> Result<i64> {
		self.watches.count(user_id).await
	}

	pub async fn delete(&self, user: &User, id: &str) -> Result<()> {
		if self.watches.delete_owned(&user.id, id).await? {
			Ok(())
		} else {
			Err(Error::NotFound(format!("watch {id}")))
		}
	}

	pub async fn promote(&self, user: &User, id: &str) -> Result<crate::models::Request> {
		let watch = self
			.watches
			.find_for_user(&user.id, id)
			.await?
			.ok_or_else(|| Error::NotFound(format!("watch {id}")))?;
		let request = self.requests.create_from_watch(user, watch).await?;
		self.watches.delete_owned(&user.id, id).await?;
		Ok(request)
	}
}
