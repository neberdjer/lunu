use aes_gcm::aead::Aead;
use aes_gcm::{AeadCore, Aes256Gcm, Key, KeyInit, Nonce};
use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use hkdf::Hkdf;
use rand::rngs::OsRng;
use sha2::Sha256;

use crate::consts::crypto::HKDF_SALT;
use crate::{Error, Result};

const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;

#[derive(Clone)]
pub struct Encryptor {
	cipher: Aes256Gcm,
}

impl Encryptor {
	pub fn new(master_key: &str, context: &[u8]) -> Result<Self> {
		let hkdf = Hkdf::<Sha256>::new(Some(HKDF_SALT), master_key.as_bytes());
		let mut key = [0u8; KEY_LEN];
		hkdf.expand(context, &mut key)
			.map_err(|error| Error::Internal(format!("key derivation failed: {error}")))?;

		Ok(Self {
			cipher: Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key)),
		})
	}

	pub fn encrypt(&self, plaintext: &str) -> Result<String> {
		Ok(STANDARD.encode(self.seal(plaintext)?))
	}

	pub fn decrypt(&self, encoded: &str) -> Result<String> {
		let combined = STANDARD
			.decode(encoded)
			.map_err(|error| Error::Internal(format!("invalid ciphertext encoding: {error}")))?;
		self.open(&combined)
	}

	pub fn encrypt_token(&self, plaintext: &str) -> Result<String> {
		Ok(URL_SAFE_NO_PAD.encode(self.seal(plaintext)?))
	}

	pub fn decrypt_token(&self, encoded: &str) -> Result<String> {
		let combined = URL_SAFE_NO_PAD
			.decode(encoded)
			.map_err(|error| Error::Internal(format!("invalid token encoding: {error}")))?;
		self.open(&combined)
	}

	fn seal(&self, plaintext: &str) -> Result<Vec<u8>> {
		let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
		let ciphertext = self
			.cipher
			.encrypt(&nonce, plaintext.as_bytes())
			.map_err(|error| Error::Internal(format!("encryption failed: {error}")))?;

		let mut combined = Vec::with_capacity(NONCE_LEN + ciphertext.len());
		combined.extend_from_slice(nonce.as_slice());
		combined.extend_from_slice(&ciphertext);
		Ok(combined)
	}

	fn open(&self, combined: &[u8]) -> Result<String> {
		if combined.len() <= NONCE_LEN {
			return Err(Error::Internal("ciphertext is too short".to_string()));
		}

		let (nonce_bytes, ciphertext) = combined.split_at(NONCE_LEN);
		let nonce = Nonce::from_slice(nonce_bytes);

		let plaintext = self
			.cipher
			.decrypt(nonce, ciphertext)
			.map_err(|error| Error::Internal(format!("decryption failed: {error}")))?;

		String::from_utf8(plaintext)
			.map_err(|error| Error::Internal(format!("decrypted value is not valid utf8: {error}")))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn encrypts_and_decrypts_roundtrip() {
		let encryptor = Encryptor::new("a-sufficiently-long-master-key", b"test-context").unwrap();
		let secret = "qbittorrent-password-123";
		let encrypted = encryptor.encrypt(secret).unwrap();
		assert_ne!(encrypted, secret);
		assert_eq!(encryptor.decrypt(&encrypted).unwrap(), secret);
	}

	#[test]
	fn a_token_round_trips_and_is_url_safe() {
		let encryptor = Encryptor::new("a-sufficiently-long-master-key", b"unsubscribe").unwrap();
		let token = encryptor.encrypt_token("user-abc-123").unwrap();
		assert!(
			token
				.chars()
				.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
			"a token must be safe to drop into a url path: {token}"
		);
		assert_eq!(encryptor.decrypt_token(&token).unwrap(), "user-abc-123");
		assert!(encryptor.decrypt_token("tampered").is_err());
	}

	#[test]
	fn nonce_makes_ciphertext_non_deterministic() {
		let encryptor = Encryptor::new("a-sufficiently-long-master-key", b"test-context").unwrap();
		assert_ne!(
			encryptor.encrypt("value").unwrap(),
			encryptor.encrypt("value").unwrap()
		);
	}

	#[test]
	fn wrong_key_fails_to_decrypt() {
		let a = Encryptor::new("master-key-number-one", b"test-context").unwrap();
		let b = Encryptor::new("master-key-number-two", b"test-context").unwrap();
		let encrypted = a.encrypt("value").unwrap();
		assert!(b.decrypt(&encrypted).is_err());
	}
}
