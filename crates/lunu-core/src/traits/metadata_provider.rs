use async_trait::async_trait;

use crate::Result;
use crate::models::{Book, Chapters, ExternalId, IdScheme, SeriesSummary};

#[async_trait]
pub trait MetadataProvider: Send + Sync {
	fn id(&self) -> &'static str;
	fn accepts(&self) -> &[IdScheme];
	async fn search(&self, query: &str, region: &str, page: i64) -> Result<Vec<Book>>;
	async fn get_book(&self, id: &ExternalId, region: &str) -> Result<Option<Book>>;
	async fn get_chapters(&self, id: &ExternalId, region: &str) -> Result<Option<Chapters>>;
	async fn similar(&self, id: &ExternalId, region: &str) -> Result<Vec<Book>>;
	async fn books_by_author(&self, author: &ExternalId, region: &str) -> Result<Vec<Book>>;
	async fn search_series(&self, query: &str, region: &str) -> Result<Vec<SeriesSummary>>;
	async fn series_books(
		&self,
		name: &str,
		id: Option<&ExternalId>,
		region: &str,
	) -> Result<Vec<Book>>;
}
