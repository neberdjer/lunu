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

pub fn hash_token(token: &str) -> String {
	hex::encode(Sha256::digest(token.as_bytes()))
}

#[cfg(test)]
mod tests {
	use super::*;

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
