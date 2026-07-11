const BTIH_MARKER: &str = "xt=urn:btih:";

pub fn info_hash(url: &str) -> Option<String> {
	let lowered = url.to_ascii_lowercase();
	let start = lowered.find(BTIH_MARKER)? + BTIH_MARKER.len();
	let hash: String = lowered[start..].chars().take_while(|c| *c != '&').collect();
	if hash.is_empty() { None } else { Some(hash) }
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn extracts_hash_from_magnet() {
		let url = "magnet:?xt=urn:btih:C12FE1C06BBA254A9DC9F519B335AA7C1367A88A&dn=book&tr=udp://x";
		assert_eq!(
			info_hash(url).as_deref(),
			Some("c12fe1c06bba254a9dc9f519b335aa7c1367a88a")
		);
	}

	#[test]
	fn hash_without_trailing_params() {
		let url = "magnet:?xt=urn:btih:abcdef0123456789";
		assert_eq!(info_hash(url).as_deref(), Some("abcdef0123456789"));
	}

	#[test]
	fn none_for_non_magnet() {
		assert_eq!(
			info_hash("https://tracker.example/download/123.torrent"),
			None
		);
	}
}
