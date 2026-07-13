use async_trait::async_trait;

use crate::Result;
use crate::models::{Book, Chapters, SeriesSummary};

#[async_trait]
pub trait MetadataProvider: Send + Sync {
	fn id(&self) -> &'static str;
	async fn search(&self, query: &str, region: &str, page: i64) -> Result<Vec<Book>>;
	async fn get_book(&self, asin: &str, region: &str) -> Result<Option<Book>>;
	async fn get_chapters(&self, asin: &str, region: &str) -> Result<Option<Chapters>>;
	async fn similar(&self, asin: &str, region: &str) -> Result<Vec<Book>>;
	async fn books_by_author(&self, author_asin: &str, region: &str) -> Result<Vec<Book>>;
	async fn search_series(&self, query: &str, region: &str) -> Result<Vec<SeriesSummary>>;
	async fn series_books(&self, name: &str, asin: Option<&str>, region: &str)
	-> Result<Vec<Book>>;
}
