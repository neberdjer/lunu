use async_trait::async_trait;

use crate::Result;
use crate::models::{ExternalId, Work};

#[async_trait]
pub trait WorkRepo: Send + Sync {
	async fn insert(&self, work: &Work) -> Result<()>;
	async fn find_by_id(&self, id: &str) -> Result<Option<Work>>;
	async fn find_by_external_id(&self, id: &ExternalId) -> Result<Option<String>>;
	async fn find_by_external_ids(&self, ids: &[ExternalId]) -> Result<Vec<(ExternalId, String)>>;
	async fn find_unidentified_by_title(
		&self,
		title: &str,
		author: Option<&str>,
	) -> Result<Option<String>>;
	async fn link_external_id(&self, work_id: &str, id: &ExternalId) -> Result<()>;
	async fn link_external_id_if_absent(&self, work_id: &str, id: &ExternalId) -> Result<()>;
	async fn external_ids(&self, work_id: &str) -> Result<Vec<ExternalId>>;
}
