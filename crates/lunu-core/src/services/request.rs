use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};

use crate::consts::reasons;
use crate::models::{
	GrabPayload, JobType, NotificationEvent, NotificationKind, Request, RequestStatus, User,
	UserSettings,
};
use crate::repo::{DownloadRepo, MediaRepo, RequestRepo, UserSettingsRepo};
use crate::services::{
	ActivityService, JobService, MetadataService, NotificationInboxService, new_id, nonempty,
};
use crate::{Error, Result};

#[derive(Debug, Clone, Default)]
pub struct NewRequest {
	pub asin: String,
	pub notes: Option<String>,
	pub quality_profile_id: Option<String>,
}

impl NewRequest {
	pub fn new(asin: impl Into<String>) -> Self {
		Self {
			asin: asin.into(),
			notes: None,
			quality_profile_id: None,
		}
	}
}

pub struct RequestService {
	requests: Arc<dyn RequestRepo>,
	user_settings: Arc<dyn UserSettingsRepo>,
	metadata: Arc<MetadataService>,
	jobs: Arc<JobService>,
	activity: Arc<ActivityService>,
	downloads: Arc<dyn DownloadRepo>,
	media: Arc<dyn MediaRepo>,
	inbox: Arc<NotificationInboxService>,
}

impl RequestService {
	#[allow(clippy::too_many_arguments)]
	pub fn new(
		requests: Arc<dyn RequestRepo>,
		user_settings: Arc<dyn UserSettingsRepo>,
		metadata: Arc<MetadataService>,
		jobs: Arc<JobService>,
		activity: Arc<ActivityService>,
		downloads: Arc<dyn DownloadRepo>,
		media: Arc<dyn MediaRepo>,
		inbox: Arc<NotificationInboxService>,
	) -> Self {
		Self {
			requests,
			user_settings,
			metadata,
			jobs,
			activity,
			downloads,
			media,
			inbox,
		}
	}

	pub async fn delete(&self, caller: &User, id: &str) -> Result<()> {
		self.get_for(caller, id).await?;
		self.downloads.delete_for_request(id).await?;
		self.activity.delete_for_request(id).await?;
		self.requests.delete(id).await
	}

	pub async fn retry(&self, caller: &User, id: &str) -> Result<Request> {
		let mut request = self.get_for(caller, id).await?;
		if request.status != RequestStatus::Failed {
			return Err(Error::Conflict(reasons::REQUEST_NOT_RETRYABLE.to_string()));
		}
		request.status = RequestStatus::Approved;
		request.updated_at = Utc::now();
		self.persist(&request, None, Some(&caller.id)).await?;
		self.enqueue_fulfillment(id).await?;
		Ok(request)
	}

	async fn enqueue_fulfillment(&self, request_id: &str) -> Result<()> {
		let payload = GrabPayload {
			request_id: request_id.to_string(),
		};
		self.jobs
			.enqueue_for(JobType::Grab, &payload, request_id)
			.await?;
		Ok(())
	}

	async fn persist(
		&self,
		request: &Request,
		detail: Option<&str>,
		actor: Option<&str>,
	) -> Result<()> {
		self.requests.update(request).await?;
		self.record_status(request, detail, actor).await
	}

	async fn record_status(
		&self,
		request: &Request,
		detail: Option<&str>,
		actor: Option<&str>,
	) -> Result<()> {
		self.activity
			.record(&request.id, request.status.as_str(), detail, actor)
			.await?;
		self.notify_status(request).await;
		Ok(())
	}

	async fn notify_status(&self, request: &Request) {
		let kind = match request.status {
			RequestStatus::Pending => NotificationKind::RequestPending,
			RequestStatus::Approved => NotificationKind::RequestApproved,
			RequestStatus::Declined => NotificationKind::RequestDeclined,
			RequestStatus::Available => NotificationKind::RequestAvailable,
			RequestStatus::Failed => NotificationKind::RequestFailed,
			_ => return,
		};
		let event = NotificationEvent {
			kind,
			request_id: request.id.clone(),
			title: request.title.clone(),
			user_id: request.user_id.clone(),
		};
		let _ = self.inbox.fan_out(&event).await;
		let _ = self
			.jobs
			.enqueue_for(JobType::Notify, &event, &request.id)
			.await;
	}

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

	pub async fn list(&self) -> Result<Vec<Request>> {
		self.requests.list().await
	}

	pub async fn list_for_user(&self, user_id: &str) -> Result<Vec<Request>> {
		self.requests.list_for_user(user_id).await
	}

	pub async fn status_by_asin(&self, user_id: &str) -> Result<HashMap<String, RequestStatus>> {
		let mut statuses = HashMap::new();
		for request in self.requests.list_for_user(user_id).await? {
			statuses.entry(request.asin).or_insert(request.status);
		}
		Ok(statuses)
	}

	pub async fn status_for_asin(
		&self,
		user_id: &str,
		asin: &str,
	) -> Result<Option<RequestStatus>> {
		Ok(self
			.requests
			.find_by_user_and_asin(user_id, asin)
			.await?
			.map(|request| request.status))
	}

	pub async fn list_page(
		&self,
		caller: &User,
		status: Option<RequestStatus>,
		limit: i64,
		offset: i64,
	) -> Result<Vec<Request>> {
		self.requests
			.list_page(
				Self::scope(caller),
				status.map(|status| status.as_str()),
				limit,
				offset,
			)
			.await
	}

	pub async fn count(&self, caller: &User, status: Option<RequestStatus>) -> Result<i64> {
		self.requests
			.count(Self::scope(caller), status.map(|status| status.as_str()))
			.await
	}

	fn scope(caller: &User) -> Option<&str> {
		if caller.role.is_admin() {
			None
		} else {
			Some(caller.id.as_str())
		}
	}

	pub async fn get(&self, id: &str) -> Result<Option<Request>> {
		self.requests.find_by_id(id).await
	}

	pub async fn get_for(&self, caller: &User, id: &str) -> Result<Request> {
		self.requests
			.find_by_id(id)
			.await?
			.filter(|request| caller.role.is_admin() || request.user_id == caller.id)
			.ok_or_else(|| Error::NotFound(format!("request {id}")))
	}

	pub async fn approve(&self, admin_id: &str, id: &str) -> Result<Request> {
		self.transition(id, RequestStatus::Approved, admin_id, None)
			.await
	}

	pub async fn mark_downloading(&self, id: &str) -> Result<Request> {
		self.set_status(id, RequestStatus::Downloading, None).await
	}

	pub async fn mark_importing(&self, id: &str) -> Result<Request> {
		self.set_status(id, RequestStatus::Importing, None).await
	}

	pub async fn mark_failed(&self, id: &str, reason: Option<&str>) -> Result<Request> {
		self.set_status(id, RequestStatus::Failed, reason).await
	}

	pub async fn mark_available(&self, id: &str) -> Result<Request> {
		self.set_status(id, RequestStatus::Available, None).await
	}

	async fn set_status(
		&self,
		id: &str,
		status: RequestStatus,
		detail: Option<&str>,
	) -> Result<Request> {
		let mut request = self
			.requests
			.find_by_id(id)
			.await?
			.ok_or_else(|| Error::NotFound(format!("request {id}")))?;

		request.status = status;
		request.updated_at = Utc::now();
		self.persist(&request, detail, None).await?;
		Ok(request)
	}

	pub async fn decline(
		&self,
		admin_id: &str,
		id: &str,
		reason: Option<String>,
	) -> Result<Request> {
		let reason = nonempty(reason);
		self.transition(id, RequestStatus::Declined, admin_id, reason.as_deref())
			.await
	}

	async fn transition(
		&self,
		id: &str,
		status: RequestStatus,
		admin_id: &str,
		detail: Option<&str>,
	) -> Result<Request> {
		let mut request = self
			.requests
			.find_by_id(id)
			.await?
			.ok_or_else(|| Error::NotFound(format!("request {id}")))?;

		if !request.status.is_pending() {
			return Err(Error::Conflict(reasons::REQUEST_NOT_PENDING.to_string()));
		}

		request.status = status;
		request.approved_by = Some(admin_id.to_string());
		request.updated_at = Utc::now();
		self.persist(&request, detail, Some(admin_id)).await?;
		if status == RequestStatus::Approved {
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
