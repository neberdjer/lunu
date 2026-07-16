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

pub fn best_match(title: &str, author: Option<&str>, books: &[Book]) -> Option<(usize, MatchedBy)> {
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
	candidates
		.iter()
		.map(|(index, candidate)| (similarity(candidate, &title), *index))
		.filter(|(score, _)| *score >= MATCH_CONFIDENCE_FLOOR)
		.max_by(|a, b| a.0.total_cmp(&b.0))
		.map(|(_, index)| (index, MatchedBy::Fuzzy))
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
		let hit = best_match("The Hobbit", Some("Tolkien"), &books);
		assert_eq!(hit, Some((1, MatchedBy::Title)));
	}

	#[test]
	fn an_ambiguous_exact_title_without_an_author_matches_nothing() {
		let books = [
			titled("Dune", "Frank Herbert"),
			titled("Dune", "Brian Herbert"),
		];
		assert_eq!(best_match("Dune", None, &books), None);
		assert_eq!(
			best_match("Dune", Some("Frank Herbert"), &books),
			Some((0, MatchedBy::Title))
		);
	}

	#[test]
	fn a_fuzzy_match_requires_an_author_and_the_floor() {
		let books = [titled("The Hobit", "Tolkien")];
		assert_eq!(
			best_match("The Hobbit", Some("Tolkien"), &books),
			Some((0, MatchedBy::Fuzzy))
		);
		assert_eq!(best_match("The Hobbit", None, &books), None);
		assert_eq!(
			best_match("The Silmarillion", Some("Tolkien"), &books),
			None
		);
	}
}
