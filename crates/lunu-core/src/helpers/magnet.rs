const BTIH_MARKER: &str = "xt=urn:btih:";

pub fn info_hash(url: &str) -> Option<String> {
	let lowered = url.to_ascii_lowercase();
	let start = lowered.find(BTIH_MARKER)? + BTIH_MARKER.len();
	let raw: String = url[start..].chars().take_while(|c| *c != '&').collect();
	let raw = raw.trim();

	if raw.len() == 40 && raw.chars().all(|c| c.is_ascii_hexdigit()) {
		return Some(raw.to_ascii_lowercase());
	}
	if raw.len() == 32 {
		return base32_to_hex(raw);
	}
	None
}

fn base32_to_hex(value: &str) -> Option<String> {
	let mut bytes = Vec::with_capacity(20);
	let mut buffer: u32 = 0;
	let mut bits = 0u32;
	for c in value.chars() {
		let symbol = match c.to_ascii_uppercase() {
			ch @ 'A'..='Z' => ch as u32 - 'A' as u32,
			ch @ '2'..='7' => ch as u32 - '2' as u32 + 26,
			_ => return None,
		};
		buffer = (buffer << 5) | symbol;
		bits += 5;
		if bits >= 8 {
			bits -= 8;
			bytes.push((buffer >> bits) as u8);
		}
	}
	if bytes.len() != 20 {
		return None;
	}
	Some(hex::encode(bytes))
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
		let url = "magnet:?xt=urn:btih:c12fe1c06bba254a9dc9f519b335aa7c1367a88a";
		assert_eq!(
			info_hash(url).as_deref(),
			Some("c12fe1c06bba254a9dc9f519b335aa7c1367a88a")
		);
	}

	#[test]
	fn decodes_base32_btih_to_hex() {
		let url = "magnet:?xt=urn:btih:MFRGGZDFMZTWQ2LKNNWG23TPOBYXE43U&dn=x";
		let hash = info_hash(url).expect("base32 btih decodes");
		assert_eq!(hash.len(), 40);
		assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
		assert_eq!(hash, hash.to_ascii_lowercase());
	}

	#[test]
	fn none_for_v2_or_malformed_hash() {
		assert_eq!(info_hash("magnet:?xt=urn:btih:tooshort"), None);
		assert_eq!(
			info_hash(
				"magnet:?xt=urn:btmh:1220caf1e1e41e5b6f5e3f3d9d3c5e0f0a0b0c0d0e0f00102030405060708090a0b"
			),
			None
		);
	}

	#[test]
	fn none_for_non_magnet() {
		assert_eq!(
			info_hash("https://tracker.example/download/123.torrent"),
			None
		);
	}
}
