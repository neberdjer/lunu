use std::str::FromStr;

use actix_web::{HttpResponse, get, post, web};
use lunu_core::models::{IssueStatus, IssueType};
use serde::Deserialize;

use crate::dto::IssueResponse;
use crate::error::ApiError;
use crate::extract::{AdminUser, AuthUser};
use crate::pagination::{Page, Pagination};
use crate::state::AppState;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct OpenIssueBody {
	issue_type: String,
	#[serde(default)]
	detail: Option<String>,
}

#[derive(Deserialize, utoipa::IntoParams)]
pub struct IssueListParams {
	page: Option<i64>,
	limit: Option<i64>,
	status: Option<String>,
}

#[utoipa::path(tag = "issues", responses((status = 201, description = "Issue opened", body = IssueResponse), (status = 409, description = "Request not available")))]
#[post("/requests/{id}/issues")]
pub async fn open(
	user: AuthUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
	body: web::Json<OpenIssueBody>,
) -> Result<HttpResponse, ApiError> {
	let body = body.into_inner();
	let issue_type = IssueType::from_str(&body.issue_type)?;
	let issue = state
		.issues
		.open(&user.0, &id.into_inner(), issue_type, body.detail)
		.await?;
	Ok(HttpResponse::Created().json(IssueResponse::from(&issue)))
}

#[utoipa::path(tag = "issues", responses((status = 200, description = "Issues for a request", body = Vec<IssueResponse>)))]
#[get("/requests/{id}/issues")]
pub async fn for_request(
	user: AuthUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let issues = state.issues.for_request(&user.0, &id.into_inner()).await?;
	let items: Vec<IssueResponse> = issues.iter().map(IssueResponse::from).collect();
	Ok(HttpResponse::Ok().json(items))
}

#[utoipa::path(tag = "issues", params(IssueListParams), responses((status = 200, body = Page<IssueResponse>)))]
#[get("/issues")]
pub async fn list(
	_admin: AdminUser,
	query: web::Query<IssueListParams>,
	state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
	let pagination = Pagination::resolve(query.page, query.limit);
	let status = query
		.status
		.as_deref()
		.map(IssueStatus::from_str)
		.transpose()?;

	let (issues, total) = tokio::try_join!(
		state
			.issues
			.list_page(status, pagination.limit, pagination.offset),
		state.issues.count(status),
	)?;
	let items: Vec<IssueResponse> = issues.iter().map(IssueResponse::from).collect();
	Ok(HttpResponse::Ok().json(Page::new(items, &pagination, total)))
}

#[utoipa::path(tag = "issues", responses((status = 200, description = "Issue resolved", body = IssueResponse), (status = 409, description = "Issue already resolved")))]
#[post("/issues/{id}/resolve")]
pub async fn resolve(
	admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let issue = state.issues.resolve(&admin.id, &id.into_inner()).await?;
	Ok(HttpResponse::Ok().json(IssueResponse::from(&issue)))
}
