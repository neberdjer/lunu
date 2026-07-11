use crate::helpers::format::detect_format;
use crate::models::{QualityProfile, Release, ScoredRelease};

pub fn score_release(release: &Release, profile: &QualityProfile) -> Option<i64> {
	if release.seeders < profile.min_seeders {
		return None;
	}

	let size_mb = release.size_mb();
	if profile.min_size_mb.is_some_and(|min| size_mb < min) {
		return None;
	}
	if profile.max_size_mb.is_some_and(|max| size_mb > max) {
		return None;
	}

	let format = detect_format(&release.title);
	if !profile.allowed_formats.is_empty() {
		let allowed = format.is_some_and(|found| contains_format(&profile.allowed_formats, found));
		if !allowed {
			return None;
		}
	}

	let mut score = release.seeders.saturating_mul(profile.seeder_weight);
	if let Some(found) = format
		&& let Some(position) = profile
			.preferred_formats
			.iter()
			.position(|preferred| preferred.eq_ignore_ascii_case(found))
	{
		let rank = (profile.preferred_formats.len() - position) as i64;
		score = score.saturating_add(rank.saturating_mul(profile.format_weight));
	}

	Some(score)
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
			info_url: None,
			publish_date: None,
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
}
