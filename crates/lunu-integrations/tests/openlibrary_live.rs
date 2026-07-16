use lunu_core::models::{ExternalId, IdScheme};
use lunu_core::traits::MetadataProvider;
use lunu_integrations::metadata::OpenLibraryProvider;

const REGION: &str = "us";
const HOBBIT_ISBN: &str = "9780007487295";

fn provider() -> OpenLibraryProvider {
	OpenLibraryProvider::new()
}

#[tokio::test]
#[ignore]
async fn searching_a_title_returns_that_title() {
	let books = provider()
		.search("the hobbit tolkien", REGION, 1)
		.await
		.expect("search succeeds");

	assert!(
		books.iter().any(|book| book.title.contains("Hobbit")),
		"page 1 must return the searched book, got: {:?}",
		books.iter().map(|b| &b.title).collect::<Vec<_>>()
	);
}

#[tokio::test]
#[ignore]
async fn search_results_carry_only_isbn_ids() {
	let books = provider()
		.search("the hobbit tolkien", REGION, 1)
		.await
		.unwrap();

	let identified = books.iter().filter(|book| !book.ids.is_empty()).count();
	assert!(identified > 0, "some results must be identifiable");
	for book in &books {
		for id in &book.ids {
			assert!(
				id.is(IdScheme::Isbn),
				"{} carries a non-isbn id from an isbn-only source",
				book.title
			);
		}
	}
}

#[tokio::test]
#[ignore]
async fn get_book_by_isbn_returns_the_edition() {
	let book = provider()
		.get_book(&ExternalId::isbn(HOBBIT_ISBN), REGION)
		.await
		.expect("lookup succeeds")
		.expect("the edition exists");

	assert!(book.title.contains("Hobbit"), "got {}", book.title);
	assert!(
		book.ids.contains(&ExternalId::isbn(HOBBIT_ISBN)),
		"the isbn that found it must be among its ids"
	);
	assert!(!book.authors.is_empty(), "authors must be populated");
	assert!(book.cover_url.is_some(), "cover must be populated");
}

#[tokio::test]
#[ignore]
async fn an_asin_is_politely_declined() {
	let book = provider()
		.get_book(&ExternalId::asin("1705009050"), REGION)
		.await
		.unwrap();
	assert!(
		book.is_none(),
		"an isbn-only source must not guess at an audible identifier"
	);
}

#[tokio::test]
#[ignore]
async fn an_unknown_isbn_is_absent_rather_than_an_error() {
	let book = provider()
		.get_book(&ExternalId::isbn("9799999999990"), REGION)
		.await;
	assert!(matches!(book, Ok(None)), "got {book:?}");
}
