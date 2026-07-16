use chrono::{DateTime, Duration, Utc};

use super::{NewRequest, RequestService};
use crate::consts::reasons;
use crate::models::{Book, ExternalId, Format, Request, RequestStatus, User, UserSettings};
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

	pub async fn create(&self, user: &User, input: NewRequest) -> Result<Request> {
		let book = self
			.metadata
			.get_book(&ExternalId::asin(&input.asin))
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
		let asin = input.asin.as_str();
		let (work_id, owned) =
			tokio::try_join!(self.works.for_book(&book), self.media.find_by_asin(asin))?;
		let work_id =
			work_id.ok_or_else(|| Error::Validation(reasons::INVALID_ASIN.to_string()))?;

		self.reject_duplicate(&user.id, &work_id).await?;

		let (status, quota_guard) = if owned.is_some() {
			(RequestStatus::Available, None)
		} else {
			self.approval(user).await?
		};

		let now = Utc::now();
		let request = Request {
			id: new_id(),
			user_id: user.id.clone(),
			work_id,
			format: Format::Audiobook,
			asin: Some(asin.to_string()),
			title: book.title,
			author: book.authors.into_iter().next(),
			cover_url: book.cover_url,
			status,
			approved_by: None,
			notes: nonempty(input.notes),
			quality_profile_id: input.quality_profile_id,
			created_at: now,
			updated_at: now,
		};

		self.finalize(request, quota_guard).await
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
		let (status, quota_guard) = self.approval(user).await?;

		let now = Utc::now();
		let request = Request {
			id: new_id(),
			user_id: user.id.clone(),
			work_id,
			format: Format::Audiobook,
			asin: None,
			title,
			author,
			cover_url: None,
			status,
			approved_by: None,
			notes: nonempty(input.notes),
			quality_profile_id: input.quality_profile_id,
			created_at: now,
			updated_at: now,
		};

		self.finalize(request, quota_guard).await
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
