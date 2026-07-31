pub const HKDF_SALT: &[u8] = b"lunu-hkdf-salt-v1";
pub const SETTINGS_ENCRYPTION_CONTEXT: &[u8] = b"lunu-settings-encryption-v1";
pub const MFA_ENCRYPTION_CONTEXT: &[u8] = b"lunu-mfa-secret-encryption-v1";
pub const UNSUBSCRIBE_ENCRYPTION_CONTEXT: &[u8] = b"lunu-unsubscribe-token-v1";
pub const UNSUBSCRIBE_TOKEN_MAX_AGE_DAYS: i64 = 90;
pub const TOKEN_BYTES: usize = 32;
