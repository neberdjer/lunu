use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;

use crate::consts::reasons;
use crate::models::{
	ExternalId, GrabPayload, JobType, NotificationEvent, NotificationKind, Request, RequestStatus,
	User,
};
use crate::repo::{DownloadRepo, MediaRepo, RequestRepo, UserSettingsRepo};
use crate::services::{
	ActivityService, JobService, MetadataService, NotificationInboxService, WorkService, nonempty,
};
use crate::{Error, Result};

mod create;

pub use create::ManualRequest;

#[derive(Debug, Clone)]
pub struct NewRequest {
	pub id: ExternalId,
	pub notes: Option<String>,
	pub quality_profile_id: Option<String>,
}

impl NewRequest {
	pub fn new(id: ExternalId) -> Self {
		Self {
			id,
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
	works: Arc<WorkService>,
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
		works: Arc<WorkService>,
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
			works,
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

	pub async fn status_by_asin(
		&self,
		user_id: &str,
		asins: &[String],
	) -> Result<HashMap<String, RequestStatus>> {
		let mut statuses = HashMap::new();
		for (asin, status) in self.requests.status_by_asins(user_id, asins).await? {
			statuses.entry(asin).or_insert(status);
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
		let won = self
			.requests
			.transition_if_pending(id, status.as_str(), admin_id, Utc::now())
			.await?;
		if !won {
			if self.requests.find_by_id(id).await?.is_none() {
				return Err(Error::NotFound(format!("request {id}")));
			}
			return Err(Error::Conflict(reasons::REQUEST_NOT_PENDING.to_string()));
		}

		let request = self
			.requests
			.find_by_id(id)
			.await?
			.ok_or_else(|| Error::NotFound(format!("request {id}")))?;
		self.record_status(&request, detail, Some(admin_id)).await?;
		if status == RequestStatus::Approved {
			self.enqueue_fulfillment(&request.id).await?;
		}
		Ok(request)
	}
}
