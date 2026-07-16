use std::sync::Mutex;

use lunu_core::consts::metadata::{METADATA_AUDNEXUS_ENABLED, METADATA_AUDNEXUS_PRIORITY};

use super::builders::*;
use super::*;

mod routing;

struct CountingProvider {
	id: &'static str,
	books: Vec<Book>,
	fails: bool,
	schemes: Vec<IdScheme>,
	calls: Mutex<usize>,
}

impl CountingProvider {
	fn returning(id: &'static str, titles: &[&str]) -> Self {
		Self {
			id,
			books: titles.iter().map(|title| book(title)).collect(),
			fails: false,
			schemes: vec![IdScheme::Asin],
			calls: Mutex::new(0),
		}
	}

	fn failing(id: &'static str) -> Self {
		Self {
			id,
			books: Vec::new(),
			fails: true,
			schemes: vec![IdScheme::Asin],
			calls: Mutex::new(0),
		}
	}

	fn speaking(id: &'static str, schemes: &[IdScheme]) -> Self {
		Self {
			schemes: schemes.to_vec(),
			..Self::returning(id, &["Dune"])
		}
	}

	fn calls(&self) -> usize {
		*self.calls.lock().unwrap()
	}
}

#[async_trait]
impl MetadataProvider for CountingProvider {
	fn id(&self) -> &'static str {
		self.id
	}

	fn accepts(&self) -> &[IdScheme] {
		&self.schemes
	}
	async fn search(&self, _query: &str, _region: &str, _page: i64) -> CoreResult<Vec<Book>> {
		*self.calls.lock().unwrap() += 1;
		if self.fails {
			return Err(Error::Integration("provider is down".to_string()));
		}
		Ok(self.books.clone())
	}
	async fn get_book(&self, _id: &ExternalId, _region: &str) -> CoreResult<Option<Book>> {
		*self.calls.lock().unwrap() += 1;
		if self.fails {
			return Err(Error::Integration("provider is down".to_string()));
		}
		Ok(self.books.first().cloned())
	}
	async fn get_chapters(&self, _id: &ExternalId, _region: &str) -> CoreResult<Option<Chapters>> {
		Ok(None)
	}
	async fn similar(&self, _id: &ExternalId, _region: &str) -> CoreResult<Vec<Book>> {
		Ok(Vec::new())
	}
	async fn books_by_author(&self, _author: &ExternalId, _region: &str) -> CoreResult<Vec<Book>> {
		Ok(Vec::new())
	}
	async fn search_series(&self, _query: &str, _region: &str) -> CoreResult<Vec<SeriesSummary>> {
		Ok(Vec::new())
	}
	async fn series_books(
		&self,
		_name: &str,
		_id: Option<&ExternalId>,
		_region: &str,
	) -> CoreResult<Vec<Book>> {
		Ok(Vec::new())
	}
}

fn service(db: &Db, providers: Vec<Arc<dyn MetadataProvider>>) -> MetadataService {
	MetadataService::new(
		providers,
		Arc::new(SqlxMetadataCacheRepo::new(db.clone())),
		settings_service(db),
	)
}

#[tokio::test]
async fn a_provider_is_enabled_by_default() {
	let db = memory_db().await;
	let audnexus = Arc::new(CountingProvider::returning("audnexus", &["Dune"]));
	let service = service(&db, vec![audnexus.clone()]);

	let books = service.search("dune", 1).await.unwrap();

	assert_eq!(books.len(), 1);
	assert_eq!(
		audnexus.calls(),
		1,
		"an unconfigured provider defaults to on"
	);
}

#[tokio::test]
async fn a_disabled_provider_is_never_called() {
	let db = memory_db().await;
	settings_service(&db)
		.set(METADATA_AUDNEXUS_ENABLED, "off")
		.await
		.unwrap();
	let audnexus = Arc::new(CountingProvider::returning("audnexus", &["Dune"]));
	let service = service(&db, vec![audnexus.clone()]);

	let result = service.search("dune", 1).await;

	assert!(
		matches!(result, Err(Error::Validation(_))),
		"disabling every provider must fail loudly, not return silent empty results"
	);
	assert_eq!(
		audnexus.calls(),
		0,
		"a disabled provider must not be called"
	);
}

#[tokio::test]
async fn a_failing_provider_falls_back_to_the_next() {
	let db = memory_db().await;
	let broken = Arc::new(CountingProvider::failing("broken"));
	let backup = Arc::new(CountingProvider::returning("backup", &["Dune"]));
	let service = service(&db, vec![broken.clone(), backup.clone()]);

	let books = service.search("dune", 1).await.unwrap();

	assert_eq!(books.len(), 1, "the backup answers when the first is down");
	assert_eq!(broken.calls(), 1);
	assert_eq!(backup.calls(), 1);
}

#[tokio::test]
async fn a_provider_with_no_results_falls_back_to_the_next() {
	let db = memory_db().await;
	let empty = Arc::new(CountingProvider::returning("empty", &[]));
	let backup = Arc::new(CountingProvider::returning("backup", &["Dune"]));
	let service = service(&db, vec![empty.clone(), backup.clone()]);

	let books = service.search("dune", 1).await.unwrap();

	assert_eq!(books.len(), 1);
	assert_eq!(empty.calls(), 1);
	assert_eq!(backup.calls(), 1);
}

#[tokio::test]
async fn registration_order_breaks_priority_ties() {
	let db = memory_db().await;
	let first = Arc::new(CountingProvider::returning("first", &["From First"]));
	let second = Arc::new(CountingProvider::returning("second", &["From Second"]));
	let service = service(&db, vec![first.clone(), second.clone()]);

	let books = service.search("dune", 1).await.unwrap();

	assert_eq!(books[0].title, "From First");
	assert_eq!(
		second.calls(),
		0,
		"a later provider is not consulted once an earlier one answers"
	);
}

#[tokio::test]
async fn a_lower_priority_number_is_preferred_over_registration_order() {
	let db = memory_db().await;
	settings_service(&db)
		.set(METADATA_AUDNEXUS_PRIORITY, "10")
		.await
		.unwrap();

	let registered_first = Arc::new(CountingProvider::returning("first", &["From First"]));
	let audnexus = Arc::new(CountingProvider::returning("audnexus", &["From Audnexus"]));
	let service = service(&db, vec![registered_first.clone(), audnexus.clone()]);

	let books = service.search("dune", 1).await.unwrap();

	assert_eq!(
		books[0].title, "From Audnexus",
		"audnexus at priority 10 must outrank an unconfigured provider at the default 50"
	);
	assert_eq!(
		registered_first.calls(),
		0,
		"the preferred provider answered, so the lower-ranked one is never called"
	);
}

#[tokio::test]
async fn a_higher_priority_number_is_consulted_last() {
	let db = memory_db().await;
	settings_service(&db)
		.set(METADATA_AUDNEXUS_PRIORITY, "90")
		.await
		.unwrap();

	let registered_first = Arc::new(CountingProvider::returning("first", &["From First"]));
	let audnexus = Arc::new(CountingProvider::returning("audnexus", &["From Audnexus"]));
	let service = service(&db, vec![audnexus.clone(), registered_first.clone()]);

	let books = service.search("dune", 1).await.unwrap();

	assert_eq!(
		books[0].title, "From First",
		"demoting audnexus to 90 must let the default-priority provider answer first"
	);
	assert_eq!(audnexus.calls(), 0);
}

#[tokio::test]
async fn priority_still_falls_back_when_the_preferred_provider_fails() {
	let db = memory_db().await;
	settings_service(&db)
		.set(METADATA_AUDNEXUS_PRIORITY, "10")
		.await
		.unwrap();

	let backup = Arc::new(CountingProvider::returning("backup", &["From Backup"]));
	let audnexus = Arc::new(CountingProvider::failing("audnexus"));
	let service = service(&db, vec![backup.clone(), audnexus.clone()]);

	let books = service.search("dune", 1).await.unwrap();

	assert_eq!(
		books[0].title, "From Backup",
		"preference decides who goes first, not who is allowed to answer"
	);
	assert_eq!(
		audnexus.calls(),
		1,
		"the preferred provider was tried first"
	);
	assert_eq!(backup.calls(), 1);
}

#[tokio::test]
async fn every_provider_failing_surfaces_the_error() {
	let db = memory_db().await;
	let a = Arc::new(CountingProvider::failing("a"));
	let b = Arc::new(CountingProvider::failing("b"));
	let service = service(&db, vec![a.clone(), b.clone()]);

	let result = service.search("dune", 1).await;

	assert!(
		matches!(result, Err(Error::Integration(_))),
		"an outage across every provider must not look like an empty result set"
	);
	assert_eq!(a.calls(), 1);
	assert_eq!(b.calls(), 1);
}

#[tokio::test]
async fn the_cache_is_scoped_per_provider() {
	let db = memory_db().await;
	let first = Arc::new(CountingProvider::returning("first", &["From First"]));
	let service_a = service(&db, vec![first.clone()]);
	service_a.search("dune", 1).await.unwrap();
	assert_eq!(first.calls(), 1);

	service_a.search("dune", 1).await.unwrap();
	assert_eq!(first.calls(), 1, "the second search is served from cache");

	let second = Arc::new(CountingProvider::returning("second", &["From Second"]));
	let service_b = service(&db, vec![second.clone()]);
	let books = service_b.search("dune", 1).await.unwrap();

	assert_eq!(
		books[0].title, "From Second",
		"a different provider must not read the first provider's cached rows"
	);
	assert_eq!(second.calls(), 1);
}
