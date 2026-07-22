use base32::Alphabet;
use hmac::{Hmac, Mac};
use rand::RngCore;
use rand::rngs::OsRng;
use sha1::Sha1;

use crate::consts::auth::{TOTP_DIGITS, TOTP_SECRET_BYTES, TOTP_STEP_SECONDS};
use crate::crypto::constant_time_eq;

type HmacSha1 = Hmac<Sha1>;

pub fn generate_totp_secret() -> String {
	let mut bytes = [0u8; TOTP_SECRET_BYTES];
	OsRng.fill_bytes(&mut bytes);
	base32::encode(Alphabet::Rfc4648 { padding: false }, &bytes)
}

pub fn totp_code(secret: &str, unix_seconds: u64) -> Option<String> {
	let key = base32::decode(Alphabet::Rfc4648 { padding: false }, secret)?;
	Some(code_at(&key, unix_seconds / TOTP_STEP_SECONDS))
}

pub fn totp_match_step(secret: &str, unix_seconds: u64, presented: &str) -> Option<u64> {
	let key = base32::decode(Alphabet::Rfc4648 { padding: false }, secret)?;
	let counter = unix_seconds / TOTP_STEP_SECONDS;
	let presented = presented.trim();
	[counter.wrapping_sub(1), counter, counter.wrapping_add(1)]
		.into_iter()
		.find(|step| constant_time_eq(&code_at(&key, *step), presented))
}

pub fn totp_matches(secret: &str, unix_seconds: u64, presented: &str) -> bool {
	totp_match_step(secret, unix_seconds, presented).is_some()
}

fn code_at(key: &[u8], counter: u64) -> String {
	let mut mac = HmacSha1::new_from_slice(key).expect("hmac accepts any key length");
	mac.update(&counter.to_be_bytes());
	let digest = mac.finalize().into_bytes();

	let offset = (digest[digest.len() - 1] & 0x0f) as usize;
	let binary = ((u32::from(digest[offset]) & 0x7f) << 24)
		| (u32::from(digest[offset + 1]) << 16)
		| (u32::from(digest[offset + 2]) << 8)
		| u32::from(digest[offset + 3]);
	let modulo = 10u32.pow(TOTP_DIGITS);
	format!("{:0width$}", binary % modulo, width = TOTP_DIGITS as usize)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn matches_the_rfc_6238_sha1_test_vector() {
		let secret = base32::encode(
			Alphabet::Rfc4648 { padding: false },
			b"12345678901234567890",
		);
		assert_eq!(totp_code(&secret, 59).as_deref(), Some("287082"));
		assert_eq!(totp_code(&secret, 1111111109).as_deref(), Some("081804"));
	}

	#[test]
	fn accepts_the_adjacent_step_for_clock_skew() {
		let secret = generate_totp_secret();
		let now = 1_700_000_000;
		let previous = totp_code(&secret, now - TOTP_STEP_SECONDS).unwrap();
		assert!(totp_matches(&secret, now, &previous));
	}

	#[test]
	fn rejects_a_stale_or_wrong_code() {
		let secret = generate_totp_secret();
		let now = 1_700_000_000;
		let old = totp_code(&secret, now - TOTP_STEP_SECONDS * 5).unwrap();
		assert!(!totp_matches(&secret, now, &old));
		assert!(!totp_matches(&secret, now, "000000"));
	}
}
