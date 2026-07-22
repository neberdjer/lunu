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
		min_bitrate_kbps: None,
		bitrate_weight: 0,
		allowed_languages: Vec::new(),
		freeleech_weight: 0,
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

fn release_titled(title: &str) -> Release {
	Release {
		title: title.to_string(),
		indexer: "t".to_string(),
		protocol: Protocol::Torrent,
		size: 500 * 1024 * 1024,
		seeders: 10,
		leechers: 0,
		download_url: "magnet:x".to_string(),
		info_hash: None,
		info_url: None,
		publish_date: None,
	}
}

#[test]
fn a_release_below_the_bitrate_floor_is_rejected() {
	let profile = QualityProfile {
		min_bitrate_kbps: Some(64),
		..profile()
	};
	assert!(score_release(&release_titled("The Hobbit [M4B 32kbps]"), &profile).is_none());
	assert!(score_release(&release_titled("The Hobbit [M4B 64kbps]"), &profile).is_some());
}

#[test]
fn an_unstated_bitrate_is_not_held_against_a_release() {
	let profile = QualityProfile {
		min_bitrate_kbps: Some(64),
		..profile()
	};
	assert!(
		score_release(&release_titled("The Hobbit [M4B]"), &profile).is_some(),
		"most titles omit the bitrate, so a floor must not silently reject the whole indexer"
	);
}

#[test]
fn a_higher_bitrate_outranks_a_lower_one_when_it_is_weighted() {
	let profile = QualityProfile {
		bitrate_weight: 10,
		..profile()
	};
	let low = score_release(&release_titled("The Hobbit [M4B 32kbps]"), &profile).unwrap();
	let high = score_release(&release_titled("The Hobbit [M4B 128kbps]"), &profile).unwrap();
	assert!(high > low, "got {high} vs {low}");
}

#[test]
fn a_language_outside_the_allowed_list_is_rejected() {
	let profile = QualityProfile {
		allowed_languages: vec!["en".to_string()],
		..profile()
	};
	assert!(score_release(&release_titled("Der Hobbit [German] m4b"), &profile).is_none());
	assert!(score_release(&release_titled("The Hobbit [English] m4b"), &profile).is_some());
	assert!(
		score_release(&release_titled("The Hobbit m4b"), &profile).is_some(),
		"an unstated language is the common case and must not be rejected"
	);
}

#[test]
fn freeleech_only_matters_when_the_profile_pays_for_it() {
	let neutral = profile();
	let plain = score_release(&release_titled("The Hobbit m4b"), &neutral).unwrap();
	let free = score_release(&release_titled("The Hobbit m4b [FreeLeech]"), &neutral).unwrap();
	assert_eq!(
		plain, free,
		"the default weight is zero, so it must not tip anything"
	);

	let weighted = QualityProfile {
		freeleech_weight: 500,
		..profile()
	};
	let free = score_release(&release_titled("The Hobbit m4b [FreeLeech]"), &weighted).unwrap();
	let plain = score_release(&release_titled("The Hobbit m4b"), &weighted).unwrap();
	assert_eq!(free - plain, 500);
}
