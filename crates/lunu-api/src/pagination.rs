use lunu_core::consts::api::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, utoipa::IntoParams)]
pub struct PageParams {
	pub page: Option<i64>,
	pub limit: Option<i64>,
}

pub struct Pagination {
	pub page: i64,
	pub limit: i64,
	pub offset: i64,
}

impl Pagination {
	pub fn resolve(page: Option<i64>, limit: Option<i64>) -> Self {
		let page = page.unwrap_or(1).max(1);
		let limit = limit.unwrap_or(DEFAULT_PAGE_SIZE).clamp(1, MAX_PAGE_SIZE);
		Self {
			page,
			limit,
			offset: page.saturating_sub(1).saturating_mul(limit),
		}
	}
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct Page<T> {
	pub items: Vec<T>,
	pub page: i64,
	pub limit: i64,
	pub total: i64,
}

impl<T> Page<T> {
	pub fn new(items: Vec<T>, pagination: &Pagination, total: i64) -> Self {
		Self {
			items,
			page: pagination.page,
			limit: pagination.limit,
			total,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn huge_page_does_not_overflow() {
		let p = Pagination::resolve(Some(i64::MAX), Some(100));
		assert_eq!(p.page, i64::MAX);
		assert_eq!(p.offset, i64::MAX);
	}

	#[test]
	fn resolve_clamps_page_and_limit() {
		let p = Pagination::resolve(Some(-5), Some(0));
		assert_eq!(p.page, 1);
		assert_eq!(p.limit, 1);
		assert_eq!(p.offset, 0);
	}
}
