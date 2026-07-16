use lunu_core::models::{ExternalId, IdScheme};
use lunu_core::repo::WorkRepo;

use super::super::builders::{book, work_service};
use super::super::*;
use super::repo;

#[tokio::test]
async fn a_book_with_no_asin_still_resolves_to_a_work() {
	let db = memory_db().await;
	let works = work_service(&db);

	let mut ebook = book("Some Ebook");
	ebook.ids = vec![ExternalId::isbn("9780007487295")];

	let work_id = works
		.for_book(&ebook)
		.await
		.unwrap()
		.expect("a book identified only by isbn is still a book");

	assert_eq!(
		repo(&db)
			.find_by_external_id(&ExternalId::isbn("9780007487295"))
			.await
			.unwrap()
			.as_deref(),
		Some(work_id.as_str()),
		"the isbn a non-audible source knows it by is what finds it again"
	);
}

#[tokio::test]
async fn every_id_a_source_knows_lands_on_one_work() {
	let db = memory_db().await;
	let works = work_service(&db);

	let mut hobbit = book("The Hobbit");
	hobbit.ids = vec![
		ExternalId::asin("1705009050"),
		ExternalId::isbn("9781705009055"),
	];
	let work_id = works.for_book(&hobbit).await.unwrap().unwrap();

	for id in &hobbit.ids {
		assert_eq!(
			repo(&db).find_by_external_id(id).await.unwrap().as_deref(),
			Some(work_id.as_str()),
			"{id} must reach the same work, which is what lets two sources agree"
		);
	}
}

#[tokio::test]
async fn a_book_with_no_identifiers_at_all_mints_nothing() {
	let db = memory_db().await;
	let works = work_service(&db);

	let mut nameless = book("Nameless");
	nameless.ids = Vec::new();

	assert!(
		works.for_book(&nameless).await.unwrap().is_none(),
		"a source that identifies nothing must not silently mint a work per call"
	);
}

#[tokio::test]
async fn two_sources_sharing_one_id_converge_whatever_order_they_report_it() {
	let db = memory_db().await;
	let works = work_service(&db);

	let mut from_openlibrary = book("The Hobbit");
	from_openlibrary.ids = vec![
		ExternalId::isbn("9781705009055"),
		ExternalId::new(IdScheme::Asin, "OL-ONLY"),
	];
	from_openlibrary.ids.pop();
	let first = works.for_book(&from_openlibrary).await.unwrap().unwrap();

	let mut from_audnexus = book("The Hobbit");
	from_audnexus.ids = vec![
		ExternalId::asin("1705009050"),
		ExternalId::isbn("9781705009055"),
	];
	let second = works.for_book(&from_audnexus).await.unwrap().unwrap();

	assert_eq!(
		first, second,
		"the shared isbn is not the first id audnexus reports, and resolving only the first id \
		 would mint a second work for a book we already know"
	);
	assert_eq!(
		repo(&db)
			.find_by_external_id(&ExternalId::asin("1705009050"))
			.await
			.unwrap()
			.as_deref(),
		Some(first.as_str()),
		"the newly seen asin joins the work the isbn already named"
	);
}
