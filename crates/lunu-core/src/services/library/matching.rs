use super::*;

impl LibraryService {
	pub(super) async fn identify(
		&self,
		existing: Option<&Media>,
		item: &LibraryItem,
	) -> Result<Identity> {
		if let Some(asin) = item.asin.clone() {
			let work_id = self
				.works
				.for_external_id(
					&ExternalId::asin(&asin),
					&item.title,
					item.author.as_deref(),
					item.cover_url.as_deref(),
				)
				.await?;
			return Ok(Identity {
				asin: Some(asin),
				work_id: Some(work_id),
				matched_by: Some(MatchedBy::Asin),
				region: None,
			});
		}

		if let Some(isbn) = item.isbn.as_deref()
			&& let Some(work_id) = self
				.works
				.find_by_external_id(&ExternalId::isbn(isbn))
				.await?
		{
			return Ok(Identity {
				asin: None,
				work_id: Some(work_id),
				matched_by: Some(MatchedBy::Isbn),
				region: None,
			});
		}

		if let Some(media) = existing
			&& media.matched_by.is_some_and(survives_resync)
		{
			return Ok(Identity {
				asin: media.asin.clone(),
				work_id: media.work_id.clone(),
				matched_by: media.matched_by,
				region: media.metadata_region.clone(),
			});
		}

		let Some((book, matched_by)) = self.search_match(item).await else {
			return Ok(Identity::default());
		};
		let asin = book.asin().map(str::to_string);
		let region = match &asin {
			Some(_) => Some(self.metadata.current_region().await?),
			None => None,
		};
		Ok(Identity {
			asin,
			work_id: self.works.for_book(&book).await?,
			matched_by: Some(matched_by),
			region,
		})
	}

	pub(super) async fn search_match(&self, item: &LibraryItem) -> Option<(Book, MatchedBy)> {
		let author = item.author.as_deref();
		let query = [
			Some(item.title.as_str()),
			author,
			item.series_name.as_deref(),
		]
		.into_iter()
		.flatten()
		.collect::<Vec<_>>()
		.join(" ");
		let mut books = self.metadata.search(&query, 1).await.ok()?;
		let series = item
			.series_name
			.as_deref()
			.zip(item.series_sequence.as_deref());
		let (index, matched_by) = best_match(&item.title, author, series, &books)?;
		Some((books.swap_remove(index), matched_by))
	}
}

pub(super) fn survives_resync(matched: MatchedBy) -> bool {
	matches!(
		matched,
		MatchedBy::Title | MatchedBy::Series | MatchedBy::Fuzzy | MatchedBy::Manual
	)
}

pub(super) fn newly_matched(prior: Option<MatchedBy>, new: Option<MatchedBy>) -> bool {
	prior.is_none() && new.is_some_and(survives_resync)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn only_the_matches_a_search_paid_for_survive_a_resync() {
		for matched in [
			MatchedBy::Title,
			MatchedBy::Series,
			MatchedBy::Fuzzy,
			MatchedBy::Manual,
		] {
			assert!(
				survives_resync(matched),
				"{matched:?} cost a metadata search or an admin's judgement, so a resync must not \
				 re-derive and possibly lose it"
			);
		}
		for matched in [MatchedBy::Asin, MatchedBy::Isbn] {
			assert!(
				!survives_resync(matched),
				"{matched:?} is re-derived locally on every sync, which is what lets an exact tier \
				 upgrade an earlier guess"
			);
		}
	}
}
