use super::*;

pub(crate) struct BookProvider;

#[async_trait]
impl lunu_core::traits::MetadataProvider for BookProvider {
	fn id(&self) -> &'static str {
		"stub-book"
	}
	fn accepts(&self) -> &[lunu_core::models::IdScheme] {
		&[lunu_core::models::IdScheme::Asin]
	}
	async fn search(&self, _query: &str, _region: &str, _page: i64) -> CoreResult<Vec<Book>> {
		Ok(Vec::new())
	}
	async fn get_book(
		&self,
		id: &lunu_core::models::ExternalId,
		_region: &str,
	) -> CoreResult<Option<Book>> {
		Ok(Some(Book {
			ids: vec![id.clone()],
			..book("The Hobbit")
		}))
	}
	async fn get_chapters(
		&self,
		_id: &lunu_core::models::ExternalId,
		_region: &str,
	) -> CoreResult<Option<lunu_core::models::Chapters>> {
		Ok(None)
	}
	async fn similar(
		&self,
		_id: &lunu_core::models::ExternalId,
		_region: &str,
	) -> CoreResult<Vec<Book>> {
		Ok(Vec::new())
	}
	async fn books_by_author(
		&self,
		_author: &lunu_core::models::ExternalId,
		_region: &str,
	) -> CoreResult<Vec<Book>> {
		Ok(Vec::new())
	}
	async fn search_series(
		&self,
		_query: &str,
		_region: &str,
	) -> CoreResult<Vec<lunu_core::models::SeriesSummary>> {
		Ok(Vec::new())
	}
	async fn series_books(
		&self,
		_name: &str,
		_id: Option<&lunu_core::models::ExternalId>,
		_region: &str,
	) -> CoreResult<Vec<Book>> {
		Ok(Vec::new())
	}
}

pub(crate) fn watch_service(
	db: &Db,
	jobs: Arc<JobService>,
) -> Arc<lunu_core::services::WatchService> {
	let metadata = Arc::new(MetadataService::new(
		vec![Arc::new(BookProvider)],
		Arc::new(SqlxMetadataCacheRepo::new(db.clone())),
		settings_service(db),
	));
	Arc::new(lunu_core::services::WatchService::new(
		Arc::new(SqlxWatchRepo::new(db.clone())),
		metadata.clone(),
		work_service(db),
		request_service_with_activity(db, jobs, activity_service(db)),
	))
}
