use actix_web::{HttpResponse, delete, get, post, put, web};
use lunu_core::Error;
use lunu_core::consts::scoring::{
	DEFAULT_FORMAT_WEIGHT, DEFAULT_MIN_SEEDERS, DEFAULT_SEEDER_WEIGHT,
};
use lunu_core::services::QualityProfileInput;
use serde::Deserialize;

use crate::dto::QualityProfileResponse;
use crate::error::ApiError;
use crate::extract::AdminUser;
use crate::pagination::{Page, PageParams, Pagination};
use crate::state::AppState;

fn default_min_seeders() -> i64 {
	DEFAULT_MIN_SEEDERS
}

fn default_seeder_weight() -> i64 {
	DEFAULT_SEEDER_WEIGHT
}

fn default_format_weight() -> i64 {
	DEFAULT_FORMAT_WEIGHT
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct QualityProfileBody {
	name: String,
	#[serde(default)]
	allowed_formats: Vec<String>,
	#[serde(default)]
	preferred_formats: Vec<String>,
	#[serde(default = "default_min_seeders")]
	min_seeders: i64,
	min_size_mb: Option<i64>,
	max_size_mb: Option<i64>,
	#[serde(default = "default_seeder_weight")]
	seeder_weight: i64,
	#[serde(default = "default_format_weight")]
	format_weight: i64,
	#[serde(default)]
	is_default: bool,
}

impl QualityProfileBody {
	fn into_input(self) -> QualityProfileInput {
		QualityProfileInput {
			name: self.name,
			allowed_formats: self.allowed_formats,
			preferred_formats: self.preferred_formats,
			min_seeders: self.min_seeders,
			min_size_mb: self.min_size_mb,
			max_size_mb: self.max_size_mb,
			seeder_weight: self.seeder_weight,
			format_weight: self.format_weight,
			is_default: self.is_default,
		}
	}
}

#[utoipa::path(tag = "quality-profiles", params(PageParams), responses((status = 200, body = Page<QualityProfileResponse>)))]
#[get("/quality-profiles")]
pub async fn list(
	_admin: AdminUser,
	query: web::Query<PageParams>,
	state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
	let pagination = Pagination::resolve(query.page, query.limit);
	let profiles = state
		.quality_profiles
		.list_page(pagination.limit, pagination.offset)
		.await?;
	let total = state.quality_profiles.count().await?;
	let items: Vec<QualityProfileResponse> =
		profiles.iter().map(QualityProfileResponse::from).collect();
	Ok(HttpResponse::Ok().json(Page::new(items, &pagination, total)))
}

#[utoipa::path(tag = "quality-profiles", responses((status = 200, body = QualityProfileResponse), (status = 404, description = "Not found")))]
#[get("/quality-profiles/{id}")]
pub async fn get(
	_admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	let id = id.into_inner();
	let profile = state
		.quality_profiles
		.get(&id)
		.await?
		.ok_or_else(|| Error::NotFound(format!("quality profile {id}")))?;
	Ok(HttpResponse::Ok().json(QualityProfileResponse::from(&profile)))
}

#[utoipa::path(tag = "quality-profiles", responses((status = 201, description = "Profile created", body = QualityProfileResponse)))]
#[post("/quality-profiles")]
pub async fn create(
	_admin: AdminUser,
	state: web::Data<AppState>,
	body: web::Json<QualityProfileBody>,
) -> Result<HttpResponse, ApiError> {
	let profile = state
		.quality_profiles
		.create(body.into_inner().into_input())
		.await?;
	Ok(HttpResponse::Created().json(QualityProfileResponse::from(&profile)))
}

#[utoipa::path(tag = "quality-profiles", responses((status = 200, description = "Profile updated", body = QualityProfileResponse)))]
#[put("/quality-profiles/{id}")]
pub async fn update(
	_admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
	body: web::Json<QualityProfileBody>,
) -> Result<HttpResponse, ApiError> {
	let profile = state
		.quality_profiles
		.update(&id, body.into_inner().into_input())
		.await?;
	Ok(HttpResponse::Ok().json(QualityProfileResponse::from(&profile)))
}

#[utoipa::path(tag = "quality-profiles", responses((status = 204, description = "Profile deleted")))]
#[delete("/quality-profiles/{id}")]
pub async fn delete(
	_admin: AdminUser,
	state: web::Data<AppState>,
	id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
	state.quality_profiles.delete(&id).await?;
	Ok(HttpResponse::NoContent().finish())
}
