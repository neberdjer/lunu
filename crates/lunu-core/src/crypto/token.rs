use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngCore;
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

use crate::consts::crypto::TOKEN_BYTES;

pub fn generate_token() -> String {
	let mut bytes = [0u8; TOKEN_BYTES];
	OsRng.fill_bytes(&mut bytes);
	URL_SAFE_NO_PAD.encode(bytes)
}

pub fn generate_numeric_code(digits: u32) -> String {
	let modulo = 10u64.pow(digits);
	let value = OsRng.next_u64() % modulo;
	format!("{value:0width$}", width = digits as usize)
}

pub fn hash_token(token: &str) -> String {
	hex::encode(Sha256::digest(token.as_bytes()))
}

pub fn constant_time_eq(a: &str, b: &str) -> bool {
	let a = a.as_bytes();
	let b = b.as_bytes();
	if a.len() != b.len() {
		return false;
	}
	a.iter().zip(b).fold(0, |acc, (x, y)| acc | (x ^ y)) == 0
}

pub fn pkce_challenge(verifier: &str) -> String {
	URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn pkce_challenge_matches_the_rfc_7636_vector() {
		assert_eq!(
			pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
			"E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
		);
	}

	#[test]
	fn tokens_are_unique() {
		assert_ne!(generate_token(), generate_token());
	}

	#[test]
	fn hashing_is_deterministic() {
		let token = generate_token();
		assert_eq!(hash_token(&token), hash_token(&token));
		assert_ne!(hash_token(&token), hash_token(&generate_token()));
	}
}
