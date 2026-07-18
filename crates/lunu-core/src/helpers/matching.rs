use crate::consts::library::MATCH_CONFIDENCE_FLOOR;
use crate::models::{Book, MatchedBy};

pub fn normalize(value: &str) -> String {
	value
		.split_whitespace()
		.collect::<Vec<_>>()
		.join(" ")
		.to_lowercase()
}

pub fn similarity(a: &str, b: &str) -> f64 {
	if a == b {
		return 1.0;
	}
	let a: Vec<char> = a.chars().collect();
	let b: Vec<char> = b.chars().collect();
	let longest = a.len().max(b.len());
	1.0 - (levenshtein(&a, &b) as f64 / longest as f64)
}

pub fn best_match(
	title: &str,
	author: Option<&str>,
	series: Option<(&str, &str)>,
	books: &[Book],
) -> Option<(usize, MatchedBy)> {
	let title = normalize(title);
	let author = author.map(normalize);
	let candidates: Vec<(usize, String)> = books
		.iter()
		.enumerate()
		.filter(|(_, book)| author_matches(author.as_deref(), book))
		.map(|(index, book)| (index, normalize(&book.title)))
		.collect();

	let mut exact = candidates
		.iter()
		.filter(|(_, candidate)| *candidate == title);
	if let Some((index, _)) = exact.next()
		&& (author.is_some() || exact.next().is_none())
	{
		return Some((*index, MatchedBy::Title));
	}

	author.as_ref()?;

	if let Some((name, sequence)) = series {
		let name = normalize(name);
		if let Some((index, _)) = candidates
			.iter()
			.find(|(index, _)| in_series_at(&books[*index], &name, sequence))
		{
			return Some((*index, MatchedBy::Series));
		}
	}

	candidates
		.iter()
		.map(|(index, candidate)| (similarity(candidate, &title), *index))
		.filter(|(score, _)| *score >= MATCH_CONFIDENCE_FLOOR)
		.max_by(|a, b| a.0.total_cmp(&b.0))
		.map(|(_, index)| (index, MatchedBy::Fuzzy))
}

fn in_series_at(book: &Book, normalized_name: &str, sequence: &str) -> bool {
	book.series.iter().any(|entry| {
		normalize(&entry.name) == normalized_name
			&& entry
				.position
				.as_deref()
				.is_some_and(|position| same_position(position, sequence))
	})
}

fn same_position(a: &str, b: &str) -> bool {
	match (a.trim().parse::<f64>(), b.trim().parse::<f64>()) {
		(Ok(a), Ok(b)) => a == b,
		_ => normalize(a) == normalize(b),
	}
}

fn author_matches(author: Option<&str>, book: &Book) -> bool {
	match author {
		None => true,
		Some(author) => book.authors.iter().any(|name| normalize(name) == author),
	}
}

fn levenshtein(a: &[char], b: &[char]) -> usize {
	let mut row: Vec<usize> = (0..=b.len()).collect();

	for (i, ca) in a.iter().enumerate() {
		let mut previous = row[0];
		row[0] = i + 1;
		for (j, cb) in b.iter().enumerate() {
			let cost = if ca == cb { 0 } else { 1 };
			let value = (previous + cost).min(row[j] + 1).min(row[j + 1] + 1);
			previous = row[j + 1];
			row[j + 1] = value;
		}
	}
	row[b.len()]
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::models::SeriesRef;

	fn in_series(title: &str, author: &str, series: &str, position: &str) -> Book {
		Book {
			series: vec![SeriesRef {
				name: series.to_string(),
				position: Some(position.to_string()),
				asin: None,
			}],
			..titled(title, author)
		}
	}

	fn titled(title: &str, author: &str) -> Book {
		Book {
			ids: Vec::new(),
			title: title.to_string(),
			subtitle: None,
			authors: vec![author.to_string()],
			author_asins: Vec::new(),
			narrators: Vec::new(),
			series: Vec::new(),
			description: None,
			cover_url: None,
			release_date: None,
			runtime_minutes: None,
			language: None,
			publisher: None,
			genres: Vec::new(),
			tags: Vec::new(),
			format_type: None,
			rating: None,
			is_adult: None,
		}
	}

	#[test]
	fn normalization_is_the_same_answer_on_every_backend() {
		assert_eq!(normalize("  The   Hobbit "), "the hobbit");
		assert_eq!(normalize("THE HOBBIT"), normalize("the hobbit"));
	}

	#[test]
	fn normalization_is_unicode_aware_rather_than_ascii_only() {
		assert_eq!(
			normalize("LES MIS\u{c9}RABLES"),
			"les mis\u{e9}rables",
			"the bundled sqlite lowercases ascii only, so this policy cannot live in sql"
		);
	}

	#[test]
	fn identical_titles_are_a_perfect_match() {
		assert_eq!(similarity("the hobbit", "the hobbit"), 1.0);
		assert_eq!(similarity("", ""), 1.0);
	}

	#[test]
	fn a_typo_stays_above_the_floor_a_different_book_falls_below() {
		assert!(
			similarity("the hobit", "the hobbit") > MATCH_CONFIDENCE_FLOOR,
			"one dropped letter is the same book"
		);
		assert!(
			similarity("the hobbit", "the two towers") < 0.5,
			"a different title must not sneak over any sane floor"
		);
	}

	#[test]
	fn similarity_is_symmetric() {
		let one = similarity("fellowship of the ring", "the fellowship of the ring");
		let two = similarity("the fellowship of the ring", "fellowship of the ring");
		assert_eq!(one, two);
	}

	#[test]
	fn an_exact_title_beats_a_closer_fuzzy_score() {
		let books = [
			titled("The Hobit", "Tolkien"),
			titled("The Hobbit", "Tolkien"),
		];
		let hit = best_match("The Hobbit", Some("Tolkien"), None, &books);
		assert_eq!(hit, Some((1, MatchedBy::Title)));
	}

	#[test]
	fn an_ambiguous_exact_title_without_an_author_matches_nothing() {
		let books = [
			titled("Dune", "Frank Herbert"),
			titled("Dune", "Brian Herbert"),
		];
		assert_eq!(best_match("Dune", None, None, &books), None);
		assert_eq!(
			best_match("Dune", Some("Frank Herbert"), None, &books),
			Some((0, MatchedBy::Title))
		);
	}

	#[test]
	fn a_fuzzy_match_requires_an_author_and_the_floor() {
		let books = [titled("The Hobit", "Tolkien")];
		assert_eq!(
			best_match("The Hobbit", Some("Tolkien"), None, &books),
			Some((0, MatchedBy::Fuzzy))
		);
		assert_eq!(best_match("The Hobbit", None, None, &books), None);
		assert_eq!(
			best_match("The Silmarillion", Some("Tolkien"), None, &books),
			None
		);
	}

	#[test]
	fn a_series_position_matches_a_title_the_shelf_names_differently() {
		let books = [in_series("Foundation", "Isaac Asimov", "Foundation", "1")];
		assert_eq!(
			best_match(
				"Foundation Book One",
				Some("Isaac Asimov"),
				Some(("Foundation", "1")),
				&books
			),
			Some((0, MatchedBy::Series)),
			"a shelf title too different to match still pins down by series and position"
		);
	}

	#[test]
	fn a_series_match_needs_the_position_to_agree() {
		let books = [in_series("Foundation", "Isaac Asimov", "Foundation", "1")];
		assert_eq!(
			best_match(
				"Foundation Book Two",
				Some("Isaac Asimov"),
				Some(("Foundation", "2")),
				&books
			),
			None,
			"the right series at the wrong position is a different book"
		);
	}

	#[test]
	fn a_series_match_still_obeys_the_author_filter() {
		let books = [in_series("Foundation", "Isaac Asimov", "Foundation", "1")];
		assert_eq!(
			best_match(
				"Foundation Book One",
				Some("Somebody Else"),
				Some(("Foundation", "1")),
				&books
			),
			None,
			"the right series slot under the wrong author is a different book"
		);
	}

	#[test]
	fn a_padded_or_decimal_position_still_matches_the_same_slot() {
		let books = [in_series("Foundation", "Isaac Asimov", "Foundation", "1")];
		for shelved in ["01", "1.0", " 1 "] {
			assert_eq!(
				best_match(
					"Foundation Book One",
					Some("Isaac Asimov"),
					Some(("Foundation", shelved)),
					&books
				),
				Some((0, MatchedBy::Series)),
				"a folder numbered {shelved} is the same slot as 1"
			);
		}
	}

	#[test]
	fn an_exact_title_still_outranks_a_series_position() {
		let books = [
			in_series("Foundation", "Isaac Asimov", "Foundation", "1"),
			titled("Second Foundation", "Isaac Asimov"),
		];
		assert_eq!(
			best_match(
				"Second Foundation",
				Some("Isaac Asimov"),
				Some(("Foundation", "1")),
				&books
			),
			Some((1, MatchedBy::Title)),
			"an exact title is stronger evidence than a series slot"
		);
	}

	#[test]
	fn a_series_position_outranks_a_fuzzy_title() {
		let books = [
			titled("Foundatian Book Onf", "Isaac Asimov"),
			in_series("Foundation", "Isaac Asimov", "Foundation", "1"),
		];
		assert_eq!(
			best_match(
				"Foundation Book One",
				Some("Isaac Asimov"),
				Some(("Foundation", "1")),
				&books
			),
			Some((1, MatchedBy::Series)),
			"an exact series slot beats a title that merely looks similar"
		);
	}
}
