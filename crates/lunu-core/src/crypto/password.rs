use argon2::Argon2;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use rand::rngs::OsRng;

use crate::{Error, Result};

pub fn hash_password(password: &str) -> Result<String> {
	let salt = SaltString::generate(&mut OsRng);
	Argon2::default()
		.hash_password(password.as_bytes(), &salt)
		.map(|hash| hash.to_string())
		.map_err(|error| Error::Internal(format!("password hashing failed: {error}")))
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
	let parsed = PasswordHash::new(hash)
		.map_err(|error| Error::Internal(format!("invalid password hash: {error}")))?;

	Ok(Argon2::default()
		.verify_password(password.as_bytes(), &parsed)
		.is_ok())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn hashes_and_verifies() {
		let hash = hash_password("correct horse battery staple").unwrap();
		assert!(verify_password("correct horse battery staple", &hash).unwrap());
		assert!(!verify_password("wrong password", &hash).unwrap());
	}

	#[test]
	fn salts_are_unique() {
		let a = hash_password("same").unwrap();
		let b = hash_password("same").unwrap();
		assert_ne!(a, b);
	}
}
