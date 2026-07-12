use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::models::{Issue, IssueStatus, IssueType};
use lunu_core::repo::IssueRepo;
use sqlx::Row;
use sqlx::any::AnyRow;

use super::{count_by_status, list_by_status, map_row_opt, map_rows};
use crate::convert::{format_dt, parse_dt, parse_enum};
use crate::{Db, db_error};

pub struct SqlxIssueRepo {
	db: Db,
}

impl SqlxIssueRepo {
	pub fn new(db: Db) -> Self {
		Self { db }
	}
}

fn map_issue(row: &AnyRow) -> Result<Issue> {
	let issue_type: String = row.try_get("issue_type").map_err(db_error)?;
	let status: String = row.try_get("status").map_err(db_error)?;
	let created_at: String = row.try_get("created_at").map_err(db_error)?;
	let updated_at: String = row.try_get("updated_at").map_err(db_error)?;

	Ok(Issue {
		id: row.try_get("id").map_err(db_error)?,
		request_id: row.try_get("request_id").map_err(db_error)?,
		reporter_id: row.try_get("reporter_id").map_err(db_error)?,
		issue_type: parse_enum::<IssueType>(&issue_type)?,
		detail: row.try_get("detail").map_err(db_error)?,
		status: parse_enum::<IssueStatus>(&status)?,
		resolved_by: row.try_get("resolved_by").map_err(db_error)?,
		created_at: parse_dt(&created_at)?,
		updated_at: parse_dt(&updated_at)?,
	})
}

#[async_trait]
impl IssueRepo for SqlxIssueRepo {
	async fn create(&self, issue: &Issue) -> Result<()> {
		sqlx::query(
			"INSERT INTO issues \
			 (id, request_id, reporter_id, issue_type, detail, status, resolved_by, created_at, updated_at) \
			 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
		)
		.bind(&issue.id)
		.bind(&issue.request_id)
		.bind(&issue.reporter_id)
		.bind(issue.issue_type.as_str())
		.bind(issue.detail.as_deref())
		.bind(issue.status.as_str())
		.bind(issue.resolved_by.as_deref())
		.bind(format_dt(issue.created_at))
		.bind(format_dt(issue.updated_at))
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(())
	}

	async fn find_by_id(&self, id: &str) -> Result<Option<Issue>> {
		let row = sqlx::query("SELECT * FROM issues WHERE id = $1")
			.bind(id)
			.fetch_optional(&self.db)
			.await
			.map_err(db_error)?;
		map_row_opt(row, map_issue)
	}

	async fn list_page(&self, status: Option<&str>, limit: i64, offset: i64) -> Result<Vec<Issue>> {
		list_by_status(&self.db, "issues", status, limit, offset, map_issue).await
	}

	async fn count(&self, status: Option<&str>) -> Result<i64> {
		count_by_status(&self.db, "issues", status).await
	}

	async fn for_request(&self, request_id: &str) -> Result<Vec<Issue>> {
		let rows =
			sqlx::query("SELECT * FROM issues WHERE request_id = $1 ORDER BY created_at DESC")
				.bind(request_id)
				.fetch_all(&self.db)
				.await
				.map_err(db_error)?;
		map_rows(rows, map_issue)
	}

	async fn update(&self, issue: &Issue) -> Result<()> {
		sqlx::query(
			"UPDATE issues SET status = $1, resolved_by = $2, updated_at = $3 WHERE id = $4",
		)
		.bind(issue.status.as_str())
		.bind(issue.resolved_by.as_deref())
		.bind(format_dt(issue.updated_at))
		.bind(&issue.id)
		.execute(&self.db)
		.await
		.map_err(db_error)?;
		Ok(())
	}
}
