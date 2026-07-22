use std::sync::OnceLock;

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

pub async fn hash_password_async(password: &str) -> Result<String> {
	let password = password.to_string();
	offload(move || hash_password(&password)).await
}

pub async fn verify_password_async(password: &str, hash: &str) -> Result<bool> {
	let password = password.to_string();
	let hash = hash.to_string();
	offload(move || verify_password(&password, &hash)).await
}

pub async fn dummy_verify_async(password: &str) {
	let password = password.to_string();
	let _ = offload(move || {
		dummy_verify(&password);
		Ok(())
	})
	.await;
}

async fn offload<T, F>(work: F) -> Result<T>
where
	F: FnOnce() -> Result<T> + Send + 'static,
	T: Send + 'static,
{
	tokio::task::spawn_blocking(work)
		.await
		.map_err(|error| Error::Internal(format!("password work did not complete: {error}")))?
}

pub fn dummy_verify(password: &str) {
	static DUMMY_HASH: OnceLock<String> = OnceLock::new();
	let hash = DUMMY_HASH
		.get_or_init(|| hash_password("lunu-timing-equalizer").expect("dummy hash builds"));
	let _ = verify_password(password, hash);
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
