use std::sync::Arc;

use lunu_core::models::{Book, Chapters, LibraryItem, SeriesRef, SeriesSummary};
use lunu_core::repo::MediaRepo;
use lunu_core::services::{LibraryService, MetadataService};
use lunu_core::traits::{LibrarySource, MetadataProvider};

use super::builders::*;
use super::*;

struct StubSource(Vec<LibraryItem>);

#[async_trait]
impl LibrarySource for StubSource {
	async fn list_items(&self) -> CoreResult<Vec<LibraryItem>> {
		Ok(self.0.clone())
	}
}

struct BookProvider;

#[async_trait]
impl MetadataProvider for BookProvider {
	fn id(&self) -> &'static str {
		"book-stub"
	}
	async fn search(&self, _query: &str, _region: &str, _page: i64) -> CoreResult<Vec<Book>> {
		Ok(Vec::new())
	}
	async fn get_book(&self, asin: &str, _region: &str) -> CoreResult<Option<Book>> {
		Ok(Some(Book {
			asin: asin.to_string(),
			title: "Foundation".to_string(),
			subtitle: None,
			authors: vec!["Isaac Asimov".to_string()],
			author_asins: Vec::new(),
			narrators: Vec::new(),
			series: vec![SeriesRef {
				name: "Foundation".to_string(),
				position: Some("1".to_string()),
				asin: None,
			}],
			description: None,
			cover_url: Some("cover".to_string()),
			release_date: None,
			runtime_minutes: None,
			language: None,
			publisher: None,
			genres: Vec::new(),
		}))
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

fn item(abs_id: &str, asin: Option<&str>) -> LibraryItem {
	LibraryItem {
		abs_item_id: abs_id.to_string(),
		asin: asin.map(str::to_string),
		title: format!("Book {abs_id}"),
		author: Some("Isaac Asimov".to_string()),
		cover_url: None,
		series_name: Some("Foundation".to_string()),
		series_sequence: asin.map(|_| "1".to_string()),
		library_path: format!("/abs/{abs_id}"),
	}
}

fn library_service(db: &Db, items: Vec<LibraryItem>) -> LibraryService {
	let media_repo = Arc::new(SqlxMediaRepo::new(db.clone()));
	let metadata = Arc::new(MetadataService::new(
		Arc::new(BookProvider),
		Arc::new(SqlxMetadataCacheRepo::new(db.clone())),
		settings_service(db),
	));
	LibraryService::new(Arc::new(StubSource(items)), media_repo, metadata)
}

#[tokio::test]
async fn sync_imports_items_with_and_without_asin() {
	let db = memory_db().await;
	let service = library_service(&db, vec![item("a", Some("B01")), item("b", None)]);

	let summary = service.sync().await.unwrap();
	assert_eq!(summary.total, 2);
	assert_eq!(summary.imported, 2);

	let media = SqlxMediaRepo::new(db.clone());
	let owned = media.available_among(&["B01".to_string()]).await.unwrap();
	assert_eq!(owned, vec!["B01".to_string()]);

	let (unmatched, total) = service.list(true, 20, 0).await.unwrap();
	assert_eq!(total, 1);
	assert_eq!(unmatched[0].abs_item_id.as_deref(), Some("b"));
}

#[tokio::test]
async fn resync_after_item_gains_asin_updates_same_row() {
	let db = memory_db().await;
	let service = library_service(&db, vec![item("b", None)]);
	service.sync().await.unwrap();

	let resynced = library_service(&db, vec![item("b", Some("B01"))]);
	let summary = resynced.sync().await.unwrap();
	assert_eq!(summary.imported, 0);
	assert_eq!(summary.updated, 1);

	let media = SqlxMediaRepo::new(db.clone());
	assert_eq!(media.count().await.unwrap(), 1);
	let owned = media.available_among(&["B01".to_string()]).await.unwrap();
	assert_eq!(owned, vec!["B01".to_string()]);
}

#[tokio::test]
async fn resync_does_not_clobber_overridden_row() {
	let db = memory_db().await;
	let service = library_service(&db, vec![item("b", None)]);
	service.sync().await.unwrap();

	let (unmatched, _) = service.list(true, 20, 0).await.unwrap();
	let id = unmatched[0].id.clone();
	service.match_asin(&id, "B002V0QK4C").await.unwrap();

	let summary = service.sync().await.unwrap();
	assert_eq!(summary.skipped, 1);
	assert_eq!(summary.updated, 0);

	let media = SqlxMediaRepo::new(db.clone())
		.find_by_id(&id)
		.await
		.unwrap()
		.unwrap();
	assert_eq!(media.title, "Foundation");
	assert!(media.overridden);
}

#[tokio::test]
async fn match_assigns_asin_and_marks_overridden() {
	let db = memory_db().await;
	let service = library_service(&db, vec![item("b", None)]);
	service.sync().await.unwrap();

	let (unmatched, _) = service.list(true, 20, 0).await.unwrap();
	let id = unmatched[0].id.clone();

	let matched = service.match_asin(&id, "B002V0QK4C").await.unwrap();
	assert_eq!(matched.asin.as_deref(), Some("B002V0QK4C"));
	assert_eq!(matched.title, "Foundation");
	assert!(matched.overridden);

	let (still_unmatched, total) = service.list(true, 20, 0).await.unwrap();
	assert_eq!(total, 0);
	assert!(still_unmatched.is_empty());
}

#[tokio::test]
async fn sync_merges_duplicate_request_and_abs_rows_without_crashing() {
	let db = memory_db().await;
	library_service(&db, vec![item("b", None)])
		.sync()
		.await
		.unwrap();

	let media_repo = SqlxMediaRepo::new(db.clone());
	media_repo
		.upsert_request(&lunu_core::models::Media {
			id: "req-row".to_string(),
			asin: Some("B01".to_string()),
			abs_item_id: None,
			title: "Book b".to_string(),
			author: None,
			cover_url: None,
			series_name: None,
			series_sequence: None,
			library_path: "/lib/b".to_string(),
			source: lunu_core::models::MediaSource::Request,
			overridden: false,
			request_id: Some("r1".to_string()),
			created_at: chrono::Utc::now(),
		})
		.await
		.unwrap();
	assert_eq!(media_repo.count().await.unwrap(), 2);

	let summary = library_service(&db, vec![item("b", Some("B01"))])
		.sync()
		.await
		.unwrap();
	assert_eq!(summary.updated, 1);
	assert_eq!(media_repo.count().await.unwrap(), 1);
	let owned = media_repo
		.available_among(&["B01".to_string()])
		.await
		.unwrap();
	assert_eq!(owned, vec!["B01".to_string()]);
}
