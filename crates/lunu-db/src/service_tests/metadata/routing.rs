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
