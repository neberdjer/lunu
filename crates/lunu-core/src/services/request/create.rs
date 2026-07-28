use chrono::{DateTime, Duration, Utc};

use super::{NewRequest, RequestService};
use crate::consts::reasons;
use crate::models::{Book, Format, Request, RequestStatus, User, UserSettings};
use crate::services::{new_id, nonempty};
use crate::{Error, Result};

#[derive(Debug, Clone, Default)]
pub struct ManualRequest {
	pub title: String,
	pub author: Option<String>,
	pub notes: Option<String>,
	pub quality_profile_id: Option<String>,
}

type Approval = (RequestStatus, Option<(i64, DateTime<Utc>)>);

struct Snapshot {
	work_id: String,
	asin: Option<String>,
	title: String,
	author: Option<String>,
	cover_url: Option<String>,
	series_name: Option<String>,
	series_sequence: Option<String>,
	metadata_region: Option<String>,
}

impl RequestService {
	async fn reject_duplicate(&self, user_id: &str, work_id: &str) -> Result<()> {
		match self
			.requests
			.find_by_user_and_work(user_id, work_id)
			.await?
		{
			Some(existing) if !existing.status.is_reopenable() => {
				Err(Error::Conflict(reasons::ALREADY_REQUESTED.to_string()))
			}
			_ => Ok(()),
		}
	}

	async fn owned_media(&self, asin: Option<&str>) -> Result<Option<crate::models::Media>> {
		match asin {
			Some(asin) => self.media.find_by_asin(asin).await,
			None => Ok(None),
		}
	}

	pub async fn create(&self, user: &User, input: NewRequest) -> Result<Request> {
		let book = self
			.metadata
			.get_book(&input.id)
			.await?
			.ok_or_else(|| Error::Validation(reasons::INVALID_ASIN.to_string()))?;
		self.create_with_book(user, input, book).await
	}

	pub async fn create_with_book(
		&self,
		user: &User,
		input: NewRequest,
		book: Book,
	) -> Result<Request> {
		let asin = book.asin().map(str::to_string);
		let (work_id, owned) = tokio::try_join!(
			self.works.for_book(&book),
			self.owned_media(asin.as_deref())
		)?;
		let work_id =
			work_id.ok_or_else(|| Error::Validation(reasons::INVALID_ASIN.to_string()))?;

		self.reject_duplicate(&user.id, &work_id).await?;

		let metadata_region = if asin.is_some() {
			Some(
				self.metadata
					.region_or_current(input.id.region.clone())
					.await?,
			)
		} else {
			None
		};
		let series = book.series.into_iter().next();
		let snapshot = Snapshot {
			work_id,
			asin,
			title: book.title,
			author: book.authors.into_iter().next(),
			cover_url: book.cover_url,
			series_name: series.as_ref().map(|entry| entry.name.clone()),
			series_sequence: series.and_then(|entry| entry.position),
			metadata_region,
		};
		self.finalize_snapshot(
			user,
			snapshot,
			owned.is_some(),
			input.notes,
			input.quality_profile_id,
		)
		.await
	}

	async fn finalize_snapshot(
		&self,
		user: &User,
		snapshot: Snapshot,
		owned: bool,
		notes: Option<String>,
		quality_profile_id: Option<String>,
	) -> Result<Request> {
		let (status, quota_guard) = if owned {
			(RequestStatus::Available, None)
		} else {
			self.approval(user).await?
		};
		let now = Utc::now();
		let request = Request {
			id: new_id(),
			user_id: user.id.clone(),
			work_id: snapshot.work_id,
			format: Format::Audiobook,
			asin: snapshot.asin,
			title: snapshot.title,
			author: snapshot.author,
			cover_url: snapshot.cover_url,
			series_name: snapshot.series_name,
			series_sequence: snapshot.series_sequence,
			metadata_region: snapshot.metadata_region,
			status,
			approved_by: None,
			notes: nonempty(notes),
			quality_profile_id,
			created_at: now,
			updated_at: now,
		};
		self.finalize(request, quota_guard).await
	}

	pub async fn create_from_watch(
		&self,
		user: &User,
		watch: crate::models::Watch,
	) -> Result<Request> {
		let (_, owned) = tokio::try_join!(
			self.reject_duplicate(&user.id, &watch.work_id),
			self.owned_media(watch.asin.as_deref())
		)?;
		let snapshot = Snapshot {
			work_id: watch.work_id,
			asin: watch.asin,
			title: watch.title,
			author: watch.author,
			cover_url: watch.cover_url,
			series_name: watch.series_name,
			series_sequence: watch.series_sequence,
			metadata_region: watch.metadata_region,
		};
		self.finalize_snapshot(user, snapshot, owned.is_some(), None, None)
			.await
	}

	pub async fn create_manual(&self, user: &User, input: ManualRequest) -> Result<Request> {
		let title = nonempty(Some(input.title))
			.ok_or_else(|| Error::Validation(reasons::REQUEST_TITLE_REQUIRED.to_string()))?;

		let author = nonempty(input.author);
		let work_id = self
			.works
			.for_unidentified(&title, author.as_deref())
			.await?;
		self.reject_duplicate(&user.id, &work_id).await?;
		let snapshot = Snapshot {
			work_id,
			asin: None,
			title,
			author,
			cover_url: None,
			series_name: None,
			series_sequence: None,
			metadata_region: None,
		};
		self.finalize_snapshot(user, snapshot, false, input.notes, input.quality_profile_id)
			.await
	}

	async fn approval(&self, user: &User) -> Result<Approval> {
		if user.role.is_admin() {
			return Ok((RequestStatus::Approved, None));
		}
		let settings = self.user_settings.get(&user.id).await?;
		let guard = Self::quota_guard(settings.as_ref());
		if settings
			.as_ref()
			.is_some_and(|settings| settings.auto_approve)
		{
			Ok((RequestStatus::Approved, guard))
		} else {
			Ok((RequestStatus::Pending, guard))
		}
	}

	async fn finalize(
		&self,
		request: Request,
		quota_guard: Option<(i64, DateTime<Utc>)>,
	) -> Result<Request> {
		let created = match quota_guard {
			Some((quota, since)) => {
				self.requests
					.create_within_quota(&request, quota, since)
					.await?
			}
			None => {
				self.requests.create(&request).await?;
				true
			}
		};
		if !created {
			return Err(Error::Validation(reasons::QUOTA_EXCEEDED.to_string()));
		}

		let detail = (request.status == RequestStatus::Available).then_some("already in library");
		self.record_status(&request, detail, Some(&request.user_id))
			.await?;
		if request.status == RequestStatus::Approved {
			self.enqueue_fulfillment(&request.id).await?;
		}
		Ok(request)
	}

	fn quota_guard(settings: Option<&UserSettings>) -> Option<(i64, DateTime<Utc>)> {
		let settings = settings?;
		let quota = settings.request_quota?;
		let days = settings.quota_days?;
		if quota <= 0 {
			return None;
		}
		Some((quota, Utc::now() - Duration::days(days)))
	}
}
