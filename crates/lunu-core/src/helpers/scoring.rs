use crate::helpers::format::detect_format;
use crate::models::{QualityProfile, Release, ScoredRelease};

pub fn score_release(release: &Release, profile: &QualityProfile) -> Option<i64> {
	let swarmed = release.protocol.has_swarm();
	if swarmed && release.seeders < profile.min_seeders {
		return None;
	}

	let size_mb = release.size_mb();
	if profile.min_size_mb.is_some_and(|min| size_mb < min) {
		return None;
	}
	if profile.max_size_mb.is_some_and(|max| size_mb > max) {
		return None;
	}

	let title = format!(" {} ", tokenize(&release.title));
	if profile
		.avoided_keywords
		.iter()
		.any(|keyword| contains_keyword(&title, keyword))
	{
		return None;
	}

	let format = detect_format(&release.title);
	if !profile.allowed_formats.is_empty()
		&& format.is_some_and(|found| !contains_format(&profile.allowed_formats, found))
	{
		return None;
	}

	let mut score = if swarmed {
		release.seeders.saturating_mul(profile.seeder_weight)
	} else {
		0
	};
	if profile.preferred_protocol == Some(release.protocol) {
		score = score.saturating_add(profile.protocol_weight);
	}
	if let Some(found) = format
		&& let Some(position) = profile
			.preferred_formats
			.iter()
			.position(|preferred| preferred.eq_ignore_ascii_case(found))
	{
		let rank = (profile.preferred_formats.len() - position) as i64;
		score = score.saturating_add(rank.saturating_mul(profile.format_weight));
	}

	let hits = profile
		.preferred_keywords
		.iter()
		.filter(|keyword| contains_keyword(&title, keyword))
		.count() as i64;
	score = score.saturating_add(hits.saturating_mul(profile.keyword_weight));

	Some(score)
}

fn contains_keyword(padded_title: &str, keyword: &str) -> bool {
	let keyword = tokenize(keyword);
	!keyword.is_empty() && padded_title.contains(&format!(" {keyword} "))
}

fn tokenize(value: &str) -> String {
	value
		.to_lowercase()
		.split(|c: char| !c.is_alphanumeric())
		.filter(|token| !token.is_empty())
		.collect::<Vec<_>>()
		.join(" ")
}

pub fn rank_releases(releases: Vec<Release>, profile: &QualityProfile) -> Vec<ScoredRelease> {
	let mut scored: Vec<ScoredRelease> = releases
		.into_iter()
		.filter_map(|release| {
			let score = score_release(&release, profile)?;
			Some(ScoredRelease { release, score })
		})
		.collect();

	scored.sort_by_key(|scored| std::cmp::Reverse(scored.score));
	scored
}

fn contains_format(formats: &[String], target: &str) -> bool {
	formats
		.iter()
		.any(|format| format.eq_ignore_ascii_case(target))
}

#[cfg(test)]
mod tests {
	use chrono::Utc;

	use super::*;
	use crate::models::Protocol;

	fn profile() -> QualityProfile {
		QualityProfile {
			id: "p1".to_string(),
			name: "default".to_string(),
			allowed_formats: vec!["m4b".to_string(), "mp3".to_string()],
			preferred_formats: vec!["m4b".to_string(), "mp3".to_string()],
			min_seeders: 1,
			min_size_mb: None,
			max_size_mb: None,
			seeder_weight: 1,
			format_weight: 100,
			preferred_keywords: Vec::new(),
			avoided_keywords: Vec::new(),
			keyword_weight: 40,
			preferred_protocol: None,
			protocol_weight: 100,
			is_default: true,
			created_at: Utc::now(),
			updated_at: Utc::now(),
		}
	}

	fn release(title: &str, seeders: i64) -> Release {
		Release {
			title: title.to_string(),
			indexer: "test".to_string(),
			protocol: Protocol::Torrent,
			size: 500 * 1024 * 1024,
			seeders,
			leechers: 0,
			download_url: "magnet:?x".to_string(),
			info_hash: None,
			info_url: None,
			publish_date: None,
		}
	}

	fn usenet(title: &str) -> Release {
		Release {
			protocol: Protocol::Usenet,
			..release(title, 0)
		}
	}

	#[test]
	fn rejects_below_min_seeders() {
		let mut p = profile();
		p.min_seeders = 5;
		assert!(score_release(&release("Title [M4B]", 2), &p).is_none());
	}

	#[test]
	fn rejects_disallowed_format() {
		assert!(score_release(&release("Title [FLAC]", 10), &profile()).is_none());
	}

	#[test]
	fn a_title_that_names_no_format_stays_eligible() {
		let ranked = rank_releases(vec![release("The Hobbit - J.R.R. Tolkien", 10)], &profile());
		assert_eq!(
			ranked.len(),
			1,
			"indexers that keep the format in metadata rather than the title must not be wiped \
			 out the moment an allowed-format list is configured"
		);
	}

	#[test]
	fn ranks_preferred_format_and_seeders() {
		let ranked = rank_releases(
			vec![
				release("Book [MP3]", 100),
				release("Book [M4B]", 10),
				release("Book [FLAC]", 999),
			],
			&profile(),
		);

		assert_eq!(ranked.len(), 2);
		assert_eq!(ranked[0].release.title, "Book [M4B]");
		assert!(ranked[0].score > ranked[1].score);
	}

	#[test]
	fn an_avoided_keyword_rejects_the_release() {
		let mut p = profile();
		p.avoided_keywords = vec!["abridged".to_string(), "graphic audio".to_string()];
		assert!(score_release(&release("Title (Abridged) [M4B]", 10), &p).is_none());
		assert!(score_release(&release("Title Graphic.Audio [M4B]", 10), &p).is_none());
	}

	#[test]
	fn unabridged_survives_an_avoided_abridged_keyword() {
		let mut p = profile();
		p.avoided_keywords = vec!["abridged".to_string()];
		assert!(
			score_release(&release("Title Unabridged [M4B]", 10), &p).is_some(),
			"a keyword must match on word boundaries, not as a substring"
		);
	}

	#[test]
	fn a_preferred_keyword_outranks_a_plain_release() {
		let mut p = profile();
		p.preferred_keywords = vec!["unabridged".to_string()];
		let ranked = rank_releases(
			vec![
				release("Title [M4B]", 10),
				release("Title Unabridged [M4B]", 10),
			],
			&p,
		);
		assert_eq!(ranked[0].release.title, "Title Unabridged [M4B]");
		assert_eq!(ranked[0].score - ranked[1].score, p.keyword_weight);
	}

	#[test]
	fn blank_keywords_match_nothing() {
		let mut p = profile();
		p.avoided_keywords = vec!["".to_string(), "  ".to_string()];
		assert!(score_release(&release("Title [M4B]", 10), &p).is_some());
	}

	#[test]
	fn a_seeder_floor_meant_for_torrents_never_filters_out_usenet() {
		let mut p = profile();
		p.min_seeders = 20;
		assert!(
			score_release(&release("Title [M4B]", 5), &p).is_none(),
			"the floor still culls a weak torrent"
		);
		assert!(
			score_release(&usenet("Title [M4B]"), &p).is_some(),
			"usenet has no swarm, so a seeder floor must not silently kill every nzb"
		);
	}

	#[test]
	fn usenet_scores_without_a_phantom_seeder_bonus() {
		let p = profile();
		let nzb = score_release(&usenet("Title [M4B]"), &p).unwrap();
		let torrent = score_release(&release("Title [M4B]", 10), &p).unwrap();
		assert_eq!(
			torrent - nzb,
			10 * p.seeder_weight,
			"a torrent's edge over an nzb is exactly its swarm, not an invented one"
		);
	}

	#[test]
	fn a_preferred_protocol_lifts_it_over_a_better_seeded_rival() {
		let mut p = profile();
		p.preferred_protocol = Some(Protocol::Usenet);
		let ranked = rank_releases(vec![release("Title [M4B]", 50), usenet("Title [M4B]")], &p);
		assert_eq!(
			ranked[0].release.protocol,
			Protocol::Usenet,
			"a stated protocol preference must outweigh raw seeder count"
		);
		assert_eq!(ranked[1].score + p.protocol_weight - 50, ranked[0].score);
	}
}
