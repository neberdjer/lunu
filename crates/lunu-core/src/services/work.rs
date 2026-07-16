use std::sync::Arc;

use chrono::Utc;

use crate::Result;
use crate::models::{Book, ExternalId, Work};
use crate::repo::WorkRepo;
use crate::services::new_id;

pub struct WorkService {
	works: Arc<dyn WorkRepo>,
}

impl WorkService {
	pub fn new(works: Arc<dyn WorkRepo>) -> Self {
		Self { works }
	}

	pub async fn for_book(&self, book: &Book) -> Result<Option<String>> {
		if book.ids.is_empty() {
			return Ok(None);
		}

		let mut known = None;
		for id in &book.ids {
			known = self.works.find_by_external_id(id).await?;
			if known.is_some() {
				break;
			}
		}

		let work_id = match known {
			Some(existing) => existing,
			None => {
				self.create(
					&book.title,
					book.authors.first().map(String::as_str),
					book.cover_url.as_deref(),
				)
				.await?
			}
		};

		for id in &book.ids {
			self.works.link_external_id_if_absent(&work_id, id).await?;
		}

		Ok(Some(work_id))
	}

	pub async fn for_external_id(
		&self,
		id: &ExternalId,
		title: &str,
		author: Option<&str>,
		cover_url: Option<&str>,
	) -> Result<String> {
		if let Some(existing) = self.works.find_by_external_id(id).await? {
			return Ok(existing);
		}

		let work_id = self.create(title, author, cover_url).await?;
		self.works.link_external_id(&work_id, id).await?;
		Ok(work_id)
	}

	pub async fn for_unidentified(&self, title: &str, author: Option<&str>) -> Result<String> {
		if let Some(existing) = self.works.find_unidentified_by_title(title, author).await? {
			return Ok(existing);
		}
		self.create(title, author, None).await
	}

	async fn create(
		&self,
		title: &str,
		author: Option<&str>,
		cover_url: Option<&str>,
	) -> Result<String> {
		let work = Work {
			id: new_id(),
			title: title.to_string(),
			author: author.map(str::to_string),
			cover_url: cover_url.map(str::to_string),
			created_at: Utc::now(),
		};
		self.works.insert(&work).await?;
		Ok(work.id)
	}
}
