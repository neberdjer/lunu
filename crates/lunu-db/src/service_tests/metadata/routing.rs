use super::*;

#[tokio::test]
async fn a_provider_that_cannot_speak_the_scheme_is_never_asked() {
	let db = memory_db().await;
	let asin_only = Arc::new(CountingProvider::speaking("first", &[IdScheme::Asin]));
	let isbn_only = Arc::new(CountingProvider::speaking("second", &[IdScheme::Isbn]));
	let service = service(&db, vec![isbn_only.clone(), asin_only.clone()]);

	let book = service.get_book(&ExternalId::asin("B123")).await.unwrap();

	assert!(book.is_some(), "the asin speaker answered");
	assert_eq!(
		isbn_only.calls(),
		0,
		"an isbn-only source must be routed around, not handed an asin it cannot use"
	);
	assert_eq!(asin_only.calls(), 1);
}

#[tokio::test]
async fn an_unanswerable_scheme_fails_loudly_rather_than_looking_empty() {
	let db = memory_db().await;
	let isbn_only = Arc::new(CountingProvider::speaking("isbn-only", &[IdScheme::Isbn]));
	let service = service(&db, vec![isbn_only.clone()]);

	let result = service.get_book(&ExternalId::asin("B123")).await;

	assert!(
		matches!(result, Err(Error::Validation(_))),
		"no source can answer an asin, and that must not be reported as a book that does not exist"
	);
	assert_eq!(isbn_only.calls(), 0);
}

#[tokio::test]
async fn a_provider_speaking_both_schemes_answers_either() {
	let db = memory_db().await;
	let both = Arc::new(CountingProvider::speaking(
		"both",
		&[IdScheme::Asin, IdScheme::Isbn],
	));
	let service = service(&db, vec![both.clone()]);

	assert!(
		service
			.get_book(&ExternalId::isbn("9780007487295"))
			.await
			.unwrap()
			.is_some()
	);
	assert!(
		service
			.get_book(&ExternalId::asin("B123"))
			.await
			.unwrap()
			.is_some()
	);
	assert_eq!(both.calls(), 2);
}

#[tokio::test]
async fn a_fallback_cache_entry_does_not_outrank_the_preferred_source() {
	let db = memory_db().await;

	let outage = service(
		&db,
		vec![
			Arc::new(CountingProvider::failing("primary")),
			Arc::new(CountingProvider::returning("backup", &["From Backup"])),
		],
	);
	assert_eq!(
		outage.search("dune", 1).await.unwrap()[0].title,
		"From Backup"
	);

	let recovered = service(
		&db,
		vec![
			Arc::new(CountingProvider::returning("primary", &["From Primary"])),
			Arc::new(CountingProvider::returning("backup", &["From Backup"])),
		],
	);
	let books = recovered.search("dune", 1).await.unwrap();

	assert_eq!(
		books[0].title, "From Primary",
		"one outage must not pin queries to the fallback's cache until it expires"
	);
}

#[tokio::test]
async fn refresh_discards_the_cached_answer_and_asks_again() {
	let db = memory_db().await;
	let id = ExternalId::asin("asin-Old");

	let stale = service(
		&db,
		vec![Arc::new(CountingProvider::returning("audnexus", &["Old"]))],
	);
	assert_eq!(stale.get_book(&id).await.unwrap().unwrap().title, "Old");

	let upstream_changed = service(
		&db,
		vec![Arc::new(CountingProvider::returning("audnexus", &["New"]))],
	);
	assert_eq!(
		upstream_changed.get_book(&id).await.unwrap().unwrap().title,
		"Old",
		"the cache still answers, which is exactly the staleness refresh exists to fix"
	);

	let refreshed = upstream_changed.refresh_book(&id).await.unwrap().unwrap();
	assert_eq!(refreshed.title, "New");
	assert_eq!(
		upstream_changed.get_book(&id).await.unwrap().unwrap().title,
		"New",
		"the fresh answer is cached in turn"
	);
}

#[tokio::test]
async fn an_ids_own_region_is_honored_over_the_global_setting() {
	let db = memory_db().await;
	let provider = Arc::new(CountingProvider::returning("audnexus", &["Dune"]));
	let service = service(&db, vec![provider.clone()]);

	service
		.get_book(&ExternalId::asin("B123").in_region(Some("de".to_string())))
		.await
		.unwrap();
	assert_eq!(
		provider.last_region().as_deref(),
		Some("de"),
		"an asin resolved under de must be re-fetched under de, not the current global region"
	);

	service.get_book(&ExternalId::asin("B999")).await.unwrap();
	assert_eq!(
		provider.last_region().as_deref(),
		Some("us"),
		"a region-less id still falls back to the global default"
	);
}

#[tokio::test]
async fn a_regional_id_caches_apart_from_the_global_region() {
	let db = memory_db().await;
	let provider = Arc::new(CountingProvider::returning("audnexus", &["Dune"]));
	let service = service(&db, vec![provider.clone()]);

	service.get_book(&ExternalId::asin("B123")).await.unwrap();
	service
		.get_book(&ExternalId::asin("B123").in_region(Some("de".to_string())))
		.await
		.unwrap();

	assert_eq!(
		provider.calls(),
		2,
		"the us copy and the de copy are different cache entries, so both are fetched"
	);
}
