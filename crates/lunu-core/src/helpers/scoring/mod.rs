use crate::helpers::format::detect_format;
use crate::helpers::release_tags::{
	contains_token, detect_bitrate, detect_language, is_freeleech, tokenize,
};
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
		.any(|keyword| contains_token(&title, &tokenize(keyword)))
	{
		return None;
	}

	let format = detect_format(&release.title);
	if !profile.allowed_formats.is_empty()
		&& format.is_some_and(|found| !contains_ci(&profile.allowed_formats, found))
	{
		return None;
	}

	let bitrate = detect_bitrate(&title);
	if let Some(min) = profile.min_bitrate_kbps
		&& bitrate.is_some_and(|found| found < min)
	{
		return None;
	}

	if !profile.allowed_languages.is_empty()
		&& detect_language(&title)
			.is_some_and(|found| !contains_ci(&profile.allowed_languages, found))
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
		.filter(|keyword| contains_token(&title, &tokenize(keyword)))
		.count() as i64;
	score = score.saturating_add(hits.saturating_mul(profile.keyword_weight));

	if let Some(found) = bitrate {
		score = score.saturating_add(found.saturating_mul(profile.bitrate_weight));
	}
	if profile.freeleech_weight != 0 && is_freeleech(&title) {
		score = score.saturating_add(profile.freeleech_weight);
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

fn contains_ci(values: &[String], target: &str) -> bool {
	values
		.iter()
		.any(|value| value.trim().eq_ignore_ascii_case(target))
}

#[cfg(test)]
mod tests;
