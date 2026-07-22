use super::*;

#[tokio::test]
async fn a_provider_that_found_nothing_is_not_asked_again() {
	let db = memory_db().await;
	let provider = Arc::new(CountingProvider::returning("audnexus", &[]));
	let service = service(&db, vec![provider.clone()]);

	let first = service.get_book(&ExternalId::asin("B404")).await.unwrap();
	let second = service.get_book(&ExternalId::asin("B404")).await.unwrap();

	assert!(first.is_none() && second.is_none());
	assert_eq!(
		provider.calls(),
		1,
		"an empty answer must be remembered, or every library sync re-asks the same dead lookup"
	);
}
