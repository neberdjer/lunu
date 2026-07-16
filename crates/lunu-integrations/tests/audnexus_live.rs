use lunu_core::traits::MetadataProvider;
use lunu_integrations::metadata::AudnexusProvider;

const REGION: &str = "us";
const HOBBIT: &str = "1705009050";
const TOLKIEN: &str = "B000ARC6KA";
const LOTR_SERIES: &str = "B009CFOEGK";

fn provider() -> AudnexusProvider {
	AudnexusProvider::new()
}

#[tokio::test]
#[ignore]
async fn searching_a_title_returns_that_title() {
	let books = provider()
		.search("The Hobbit", REGION, 1)
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
async fn search_pages_do_not_overlap() {
	let first = provider().search("Tolkien", REGION, 1).await.unwrap();
	let second = provider().search("Tolkien", REGION, 2).await.unwrap();

	let overlap = first
		.iter()
		.filter(|book| second.iter().any(|other| other.asin == book.asin))
		.count();
	assert_eq!(overlap, 0, "consecutive pages must not repeat results");
}

#[tokio::test]
#[ignore]
async fn books_by_author_resolves_the_asin_to_a_name() {
	let books = provider()
		.books_by_author(TOLKIEN, REGION)
		.await
		.expect("author lookup succeeds");

	assert!(
		!books.is_empty(),
		"an author asin must yield books, not an empty list"
	);
	assert!(
		books
			.iter()
			.any(|book| book.authors.iter().any(|name| name.contains("Tolkien"))),
		"the results must actually be by the requested author"
	);
}

#[tokio::test]
#[ignore]
async fn series_books_enumerates_the_whole_series_in_order() {
	let books = provider()
		.series_books("The Lord of the Rings", Some(LOTR_SERIES), REGION)
		.await
		.expect("series lookup succeeds");

	assert!(
		books.len() >= 5,
		"the series must enumerate its members, got {}",
		books.len()
	);

	let mut asins = books.iter().map(|book| &book.asin).collect::<Vec<_>>();
	asins.sort();
	let before = asins.len();
	asins.dedup();
	assert_eq!(before, asins.len(), "no book may appear twice");

	let positions: Vec<f64> = books
		.iter()
		.filter_map(|book| {
			book.series
				.iter()
				.find(|entry| entry.asin.as_deref() == Some(LOTR_SERIES))
				.and_then(|entry| entry.position.as_deref())
				.and_then(|position| position.parse::<f64>().ok())
		})
		.collect();
	assert!(
		positions.windows(2).all(|pair| pair[0] <= pair[1]),
		"series must come back in reading order, got {positions:?}"
	);
}

#[tokio::test]
#[ignore]
async fn a_series_only_returns_its_own_members() {
	let books = provider()
		.series_books("The Lord of the Rings", Some(LOTR_SERIES), REGION)
		.await
		.unwrap();

	for book in &books {
		assert!(
			book.series
				.iter()
				.any(|entry| entry.asin.as_deref() == Some(LOTR_SERIES)),
			"{} is not in the requested series",
			book.title
		);
	}
}

#[tokio::test]
#[ignore]
async fn get_book_captures_every_field_we_map() {
	let book = provider()
		.get_book(HOBBIT, REGION)
		.await
		.expect("lookup succeeds")
		.expect("the hobbit exists");

	assert_eq!(book.asin, HOBBIT);
	assert!(!book.authors.is_empty(), "authors must be populated");
	assert!(!book.narrators.is_empty(), "narrators must be populated");
	assert!(!book.genres.is_empty(), "genres must be populated");
	assert!(book.description.is_some(), "description must be populated");
	assert!(book.cover_url.is_some(), "cover must be populated");
	assert!(book.runtime_minutes.is_some(), "runtime must be populated");
	assert!(book.isbn.is_some(), "isbn must be captured");
	assert!(book.format_type.is_some(), "format must be captured");
	assert!(book.rating.is_some(), "rating must be captured");
}

#[tokio::test]
#[ignore]
async fn descriptions_never_carry_markup() {
	let searched = provider().search("The Hobbit", REGION, 1).await.unwrap();
	for book in &searched {
		let Some(description) = &book.description else {
			continue;
		};
		assert!(
			!description.contains("<p>") && !description.contains("&amp;"),
			"{} leaks html into the description: {description}",
			book.title
		);
	}
}

#[tokio::test]
#[ignore]
async fn a_searched_book_and_a_fetched_book_agree() {
	let searched = provider()
		.search("The Hobbit", REGION, 1)
		.await
		.unwrap()
		.into_iter()
		.find(|book| book.asin == HOBBIT)
		.expect("the hobbit is on page 1");
	let fetched = provider().get_book(HOBBIT, REGION).await.unwrap().unwrap();

	assert_eq!(
		searched.release_date, fetched.release_date,
		"the same book must not carry two date formats"
	);
	assert!(
		!searched.genres.is_empty() && !fetched.genres.is_empty(),
		"genres must be populated from both sources"
	);
}

#[tokio::test]
#[ignore]
async fn get_chapters_captures_accuracy_and_brand_offsets() {
	let chapters = provider()
		.get_chapters(HOBBIT, REGION)
		.await
		.expect("lookup succeeds")
		.expect("the hobbit has chapters");

	assert!(!chapters.chapters.is_empty());
	assert!(chapters.runtime_ms.is_some());
	assert!(
		chapters.is_accurate.is_some(),
		"accuracy is the caller's only signal that these offsets are trustworthy"
	);
}

#[tokio::test]
#[ignore]
async fn an_unknown_asin_is_absent_rather_than_an_error() {
	let book = provider().get_book("B000000000", REGION).await;
	assert!(matches!(book, Ok(None)), "got {book:?}");
}
