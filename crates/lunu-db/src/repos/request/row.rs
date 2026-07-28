use lunu_core::Result;
use lunu_core::models::{Format, Request, RequestStatus};
use sqlx::Row;
use sqlx::any::AnyRow;

use crate::convert::{parse_dt, parse_enum};
use crate::db_error;

pub(super) fn map_request(row: &AnyRow) -> Result<Request> {
	let status: String = row.try_get("status").map_err(db_error)?;
	let format: String = row.try_get("format").map_err(db_error)?;
	let created_at: String = row.try_get("created_at").map_err(db_error)?;
	let updated_at: String = row.try_get("updated_at").map_err(db_error)?;

	Ok(Request {
		id: row.try_get("id").map_err(db_error)?,
		user_id: row.try_get("user_id").map_err(db_error)?,
		work_id: row.try_get("work_id").map_err(db_error)?,
		format: parse_enum::<Format>(&format)?,
		asin: row.try_get("asin").map_err(db_error)?,
		title: row.try_get("title").map_err(db_error)?,
		author: row.try_get("author").map_err(db_error)?,
		cover_url: row.try_get("cover_url").map_err(db_error)?,
		series_name: row.try_get("series_name").map_err(db_error)?,
		series_sequence: row.try_get("series_sequence").map_err(db_error)?,
		metadata_region: row.try_get("metadata_region").map_err(db_error)?,
		status: parse_enum::<RequestStatus>(&status)?,
		approved_by: row.try_get("approved_by").map_err(db_error)?,
		notes: row.try_get("notes").map_err(db_error)?,
		quality_profile_id: row.try_get("quality_profile_id").map_err(db_error)?,
		created_at: parse_dt(&created_at)?,
		updated_at: parse_dt(&updated_at)?,
	})
}

pub(super) fn request_filter(user_id: Option<&str>, status: Option<&str>) -> (String, i64) {
	let mut clauses = Vec::new();
	let mut next = 1;
	if user_id.is_some() {
		clauses.push(format!("user_id = ${next}"));
		next += 1;
	}
	if status.is_some() {
		clauses.push(format!("status = ${next}"));
		next += 1;
	}
	if clauses.is_empty() {
		(String::new(), next)
	} else {
		(format!(" WHERE {}", clauses.join(" AND ")), next)
	}
}
