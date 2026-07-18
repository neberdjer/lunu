use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use lunu_core::consts::crypto::SETTINGS_ENCRYPTION_CONTEXT;
use lunu_core::consts::metadata::METADATA_GOOGLE_BOOKS_API_KEY;
use lunu_core::crypto::Encryptor;
use lunu_core::models::{ExternalId, IdScheme, Setting};
use lunu_core::repo::SettingsRepo;
use lunu_core::services::SettingsService;
use lunu_core::traits::MetadataProvider;
use lunu_integrations::metadata::GoogleBooksProvider;

const REGION: &str = "us";
const HOBBIT_ISBN: &str = "9780007487295";

struct EnvKeySettings;

#[async_trait]
impl SettingsRepo for EnvKeySettings {
	async fn get(&self, key: &str) -> lunu_core::Result<Option<Setting>> {
		if key == METADATA_GOOGLE_BOOKS_API_KEY
			&& let Ok(value) = std::env::var("GOOGLE_BOOKS_API_KEY")
			&& !value.is_empty()
		{
			return Ok(Some(Setting {
				key: key.to_string(),
				value,
				encrypted: false,
				updated_at: Utc::now(),
			}));
		}
		Ok(None)
	}
	async fn set(&self, _setting: &Setting) -> lunu_core::Result<()> {
		Ok(())
	}
	async fn get_all(&self) -> lunu_core::Result<Vec<Setting>> {
		Ok(Vec::new())
	}
	async fn delete(&self, _key: &str) -> lunu_core::Result<()> {
		Ok(())
	}
}

fn provider() -> GoogleBooksProvider {
	let encryptor = Encryptor::new("live-test-master-key", SETTINGS_ENCRYPTION_CONTEXT).unwrap();
	let settings = Arc::new(SettingsService::new(Arc::new(EnvKeySettings), encryptor));
	GoogleBooksProvider::new(settings)
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
		"search must return the requested book, got: {:?}",
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
async fn get_book_by_isbn_returns_the_edition_with_a_description() {
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
	assert!(
		book.description.is_some(),
		"google books is worth adding for its descriptions"
	);
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
