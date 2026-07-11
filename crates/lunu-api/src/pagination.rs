use lunu_core::consts::api::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
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
			offset: (page - 1) * limit,
		}
	}
}

#[derive(Serialize)]
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
