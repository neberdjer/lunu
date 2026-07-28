use super::*;

pub(super) struct CountingProvider {
	id: &'static str,
	books: Vec<Book>,
	fails: bool,
	schemes: Vec<IdScheme>,
	calls: Mutex<usize>,
	regions: Mutex<Vec<String>>,
}

impl CountingProvider {
	pub(super) fn returning(id: &'static str, titles: &[&str]) -> Self {
		Self {
			id,
			books: titles.iter().map(|title| book(title)).collect(),
			fails: false,
			schemes: vec![IdScheme::Asin],
			calls: Mutex::new(0),
			regions: Mutex::new(Vec::new()),
		}
	}

	pub(super) fn failing(id: &'static str) -> Self {
		Self {
			id,
			books: Vec::new(),
			fails: true,
			schemes: vec![IdScheme::Asin],
			calls: Mutex::new(0),
			regions: Mutex::new(Vec::new()),
		}
	}

	pub(super) fn speaking(id: &'static str, schemes: &[IdScheme]) -> Self {
		Self {
			schemes: schemes.to_vec(),
			..Self::returning(id, &["Dune"])
		}
	}

	pub(super) fn calls(&self) -> usize {
		*self.calls.lock().unwrap()
	}

	pub(super) fn last_region(&self) -> Option<String> {
		self.regions.lock().unwrap().last().cloned()
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
	async fn get_book(&self, _id: &ExternalId, region: &str) -> CoreResult<Option<Book>> {
		*self.calls.lock().unwrap() += 1;
		self.regions.lock().unwrap().push(region.to_string());
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
