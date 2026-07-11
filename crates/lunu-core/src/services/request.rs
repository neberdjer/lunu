use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::consts::reasons;
use crate::models::{GrabPayload, JobType, Request, RequestStatus, User, UserSettings};
use crate::repo::{RequestRepo, UserSettingsRepo};
use crate::services::{ActivityService, JobService, MetadataService, new_id};
use crate::{Error, Result};

pub struct RequestService {
	requests: Arc<dyn RequestRepo>,
	user_settings: Arc<dyn UserSettingsRepo>,
	metadata: Arc<MetadataService>,
	jobs: Arc<JobService>,
	activity: Arc<ActivityService>,
}

impl RequestService {
	pub fn new(
		requests: Arc<dyn RequestRepo>,
		user_settings: Arc<dyn UserSettingsRepo>,
		metadata: Arc<MetadataService>,
		jobs: Arc<JobService>,
		activity: Arc<ActivityService>,
	) -> Self {
		Self {
			requests,
			user_settings,
			metadata,
			jobs,
			activity,
		}
	}

	async fn enqueue_fulfillment(&self, request_id: &str) -> Result<()> {
		let payload = GrabPayload {
			request_id: request_id.to_string(),
		};
		self.jobs.enqueue(JobType::Grab, &payload).await?;
		Ok(())
	}

	async fn persist(&self, request: &Request) -> Result<()> {
		self.requests.update(request).await?;
		self.activity
			.record(&request.id, request.status.as_str())
			.await?;
		Ok(())
	}

	pub async fn create(&self, user: &User, asin: &str) -> Result<Request> {
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

		let auto_approve = if user.role.is_admin() {
			true
		} else {
			let settings = self.user_settings.get(&user.id).await?;
			let approve = settings
				.as_ref()
				.is_some_and(|settings| settings.auto_approve);
			if !approve {
				self.enforce_quota(&user.id, settings.as_ref()).await?;
			}
			approve
		};

		let status = if auto_approve {
			RequestStatus::Approved
		} else {
			RequestStatus::Pending
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
			created_at: now,
			updated_at: now,
		};

		self.requests.create(&request).await?;
		self.activity
			.record(&request.id, request.status.as_str())
			.await?;
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

	pub async fn get(&self, id: &str) -> Result<Option<Request>> {
		self.requests.find_by_id(id).await
	}

	pub async fn approve(&self, admin_id: &str, id: &str) -> Result<Request> {
		self.transition(id, RequestStatus::Approved, admin_id).await
	}

	pub async fn mark_downloading(&self, id: &str) -> Result<Request> {
		self.set_status(id, RequestStatus::Downloading).await
	}

	pub async fn mark_importing(&self, id: &str) -> Result<Request> {
		self.set_status(id, RequestStatus::Importing).await
	}

	pub async fn mark_failed(&self, id: &str) -> Result<Request> {
		self.set_status(id, RequestStatus::Failed).await
	}

	pub async fn mark_available(&self, id: &str) -> Result<Request> {
		self.set_status(id, RequestStatus::Available).await
	}

	async fn set_status(&self, id: &str, status: RequestStatus) -> Result<Request> {
		let mut request = self
			.requests
			.find_by_id(id)
			.await?
			.ok_or_else(|| Error::NotFound(format!("request {id}")))?;

		request.status = status;
		request.updated_at = Utc::now();
		self.persist(&request).await?;
		Ok(request)
	}

	pub async fn decline(&self, admin_id: &str, id: &str) -> Result<Request> {
		self.transition(id, RequestStatus::Declined, admin_id).await
	}

	async fn transition(&self, id: &str, status: RequestStatus, admin_id: &str) -> Result<Request> {
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
		self.persist(&request).await?;
		if status == RequestStatus::Approved {
			self.enqueue_fulfillment(&request.id).await?;
		}
		Ok(request)
	}

	async fn enforce_quota(&self, user_id: &str, settings: Option<&UserSettings>) -> Result<()> {
		let Some(settings) = settings else {
			return Ok(());
		};

		let (Some(quota), Some(days)) = (settings.request_quota, settings.quota_days) else {
			return Ok(());
		};

		if quota <= 0 {
			return Ok(());
		}

		let since = Utc::now() - Duration::days(days);
		if self.requests.count_for_user_since(user_id, since).await? >= quota {
			return Err(Error::Validation(reasons::QUOTA_EXCEEDED.to_string()));
		}

		Ok(())
	}
}
