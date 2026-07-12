use chrono::{DateTime, Duration, Utc};

use super::{NewRequest, RequestService};
use crate::consts::reasons;
use crate::models::{Request, RequestStatus, User, UserSettings};
use crate::services::{new_id, nonempty};
use crate::{Error, Result};

impl RequestService {
	pub async fn create(&self, user: &User, input: NewRequest) -> Result<Request> {
		let asin = input.asin.as_str();
		let book = self
			.metadata
			.get_book(asin)
			.await?
			.ok_or_else(|| Error::Validation(reasons::INVALID_ASIN.to_string()))?;

		if let Some(existing) = self.requests.find_by_user_and_asin(&user.id, asin).await?
			&& !existing.status.is_reopenable()
		{
			return Err(Error::Conflict(reasons::ALREADY_REQUESTED.to_string()));
		}

		let (status, quota_guard, detail) = if self.media.find_by_asin(asin).await?.is_some() {
			(RequestStatus::Available, None, Some("already in library"))
		} else if user.role.is_admin() {
			(RequestStatus::Approved, None, None)
		} else {
			let settings = self.user_settings.get(&user.id).await?;
			if settings
				.as_ref()
				.is_some_and(|settings| settings.auto_approve)
			{
				(RequestStatus::Approved, None, None)
			} else {
				(
					RequestStatus::Pending,
					Self::quota_guard(settings.as_ref()),
					None,
				)
			}
		};

		let now = Utc::now();
		let request = Request {
			id: new_id(),
			user_id: user.id.clone(),
			asin: asin.to_string(),
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

		self.record_status(&request, detail, Some(&user.id)).await?;
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
