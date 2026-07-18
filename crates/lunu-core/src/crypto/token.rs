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

const RECOVERY_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
const RECOVERY_GROUP: usize = 5;

pub fn generate_recovery_code() -> String {
	let mut code = String::with_capacity(RECOVERY_GROUP * 2 + 1);
	for position in 0..RECOVERY_GROUP * 2 {
		if position == RECOVERY_GROUP {
			code.push('-');
		}
		let index = (OsRng.next_u32() as usize) % RECOVERY_ALPHABET.len();
		code.push(RECOVERY_ALPHABET[index] as char);
	}
	code
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

	#[test]
	fn recovery_codes_are_grouped_unambiguous_and_unique() {
		let code = generate_recovery_code();
		assert_eq!(code.len(), 11, "two groups of five plus a separator");
		assert_eq!(code.as_bytes()[5], b'-');
		assert!(
			code.chars()
				.all(|c| c == '-' || RECOVERY_ALPHABET.contains(&(c as u8))),
			"only the unambiguous alphabet, no 0/O/1/I: {code}"
		);
		assert_ne!(generate_recovery_code(), generate_recovery_code());
	}
}
