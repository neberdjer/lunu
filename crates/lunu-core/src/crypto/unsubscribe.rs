use chrono::{Duration, Utc};

use crate::consts::crypto::UNSUBSCRIBE_TOKEN_MAX_AGE_DAYS;
use crate::consts::reasons;
use crate::crypto::Encryptor;
use crate::{Error, Result};

fn invalid() -> Error {
	Error::Validation(reasons::UNSUBSCRIBE_TOKEN_INVALID.to_string())
}

pub fn mint_unsubscribe_token(encryptor: &Encryptor, user_id: &str) -> Result<String> {
	encryptor.encrypt_token(&format!("{}:{user_id}", Utc::now().timestamp()))
}

pub fn verify_unsubscribe_token(encryptor: &Encryptor, token: &str) -> Result<String> {
	let payload = encryptor.decrypt_token(token).map_err(|_| invalid())?;
	let (issued, user_id) = payload.split_once(':').ok_or_else(invalid)?;
	let issued: i64 = issued.parse().map_err(|_| invalid())?;
	let age = Utc::now().timestamp() - issued;
	if age < 0 || age > Duration::days(UNSUBSCRIBE_TOKEN_MAX_AGE_DAYS).num_seconds() {
		return Err(invalid());
	}
	Ok(user_id.to_string())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::consts::crypto::UNSUBSCRIBE_ENCRYPTION_CONTEXT;

	fn encryptor() -> Encryptor {
		Encryptor::new(
			"a-sufficiently-long-master-key",
			UNSUBSCRIBE_ENCRYPTION_CONTEXT,
		)
		.unwrap()
	}

	#[test]
	fn a_fresh_token_round_trips_to_its_user() {
		let enc = encryptor();
		let token = mint_unsubscribe_token(&enc, "user-abc:with-colon").unwrap();
		assert_eq!(
			verify_unsubscribe_token(&enc, &token).unwrap(),
			"user-abc:with-colon",
			"the user id is recovered even if it contains the delimiter"
		);
	}

	#[test]
	fn a_stale_token_is_rejected() {
		let enc = encryptor();
		let stale = enc.encrypt_token("0:user-abc").unwrap();
		assert!(verify_unsubscribe_token(&enc, &stale).is_err());
	}

	#[test]
	fn a_tampered_or_malformed_token_is_rejected() {
		let enc = encryptor();
		assert!(verify_unsubscribe_token(&enc, "not-a-token").is_err());
		assert!(
			verify_unsubscribe_token(&enc, &enc.encrypt_token("no-delimiter").unwrap()).is_err()
		);
	}
}
