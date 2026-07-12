use std::sync::Arc;

use chrono::Utc;

use crate::consts::reasons;
use crate::models::{Issue, IssueStatus, IssueType, User};
use crate::repo::IssueRepo;
use crate::services::{RequestService, new_id, nonempty};
use crate::{Error, Result};

pub struct IssueService {
	issues: Arc<dyn IssueRepo>,
	requests: Arc<RequestService>,
}

impl IssueService {
	pub fn new(issues: Arc<dyn IssueRepo>, requests: Arc<RequestService>) -> Self {
		Self { issues, requests }
	}

	pub async fn open(
		&self,
		reporter: &User,
		request_id: &str,
		issue_type: IssueType,
		detail: Option<String>,
	) -> Result<Issue> {
		let request = self.requests.get_for(reporter, request_id).await?;
		if !request.status.allows_issue() {
			return Err(Error::Conflict(reasons::REQUEST_NOT_AVAILABLE.to_string()));
		}
		let detail = nonempty(detail);
		let now = Utc::now();
		let issue = Issue {
			id: new_id(),
			request_id: request_id.to_string(),
			reporter_id: reporter.id.clone(),
			issue_type,
			detail,
			status: IssueStatus::Open,
			resolved_by: None,
			created_at: now,
			updated_at: now,
		};
		self.issues.create(&issue).await?;
		Ok(issue)
	}

	pub async fn for_request(&self, caller: &User, request_id: &str) -> Result<Vec<Issue>> {
		self.requests.get_for(caller, request_id).await?;
		self.issues.for_request(request_id).await
	}

	pub async fn list_page(
		&self,
		status: Option<IssueStatus>,
		limit: i64,
		offset: i64,
	) -> Result<Vec<Issue>> {
		self.issues
			.list_page(status.map(|status| status.as_str()), limit, offset)
			.await
	}

	pub async fn count(&self, status: Option<IssueStatus>) -> Result<i64> {
		self.issues
			.count(status.map(|status| status.as_str()))
			.await
	}

	pub async fn resolve(&self, admin_id: &str, id: &str) -> Result<Issue> {
		let mut issue = self
			.issues
			.find_by_id(id)
			.await?
			.ok_or_else(|| Error::NotFound(format!("issue {id}")))?;
		if !issue.status.is_open() {
			return Err(Error::Conflict(reasons::ISSUE_NOT_OPEN.to_string()));
		}
		issue.status = IssueStatus::Resolved;
		issue.resolved_by = Some(admin_id.to_string());
		issue.updated_at = Utc::now();
		self.issues.update(&issue).await?;
		Ok(issue)
	}
}
