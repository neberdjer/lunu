use super::*;

pub(super) struct StubProvider;

#[async_trait]
impl MetadataProvider for StubProvider {
	fn id(&self) -> &'static str {
		"stub"
	}
	async fn search(&self, _query: &str, _region: &str, _page: i64) -> CoreResult<Vec<Book>> {
		Ok(Vec::new())
	}
	async fn get_book(&self, _asin: &str, _region: &str) -> CoreResult<Option<Book>> {
		Ok(None)
	}
	async fn get_chapters(&self, _asin: &str, _region: &str) -> CoreResult<Option<Chapters>> {
		Ok(None)
	}
	async fn similar(&self, _asin: &str, _region: &str) -> CoreResult<Vec<Book>> {
		Ok(Vec::new())
	}
	async fn books_by_author(&self, _author_asin: &str, _region: &str) -> CoreResult<Vec<Book>> {
		Ok(Vec::new())
	}
	async fn search_series(&self, _query: &str, _region: &str) -> CoreResult<Vec<SeriesSummary>> {
		Ok(Vec::new())
	}
	async fn series_books(
		&self,
		_name: &str,
		_asin: Option<&str>,
		_region: &str,
	) -> CoreResult<Vec<Book>> {
		Ok(Vec::new())
	}
}
