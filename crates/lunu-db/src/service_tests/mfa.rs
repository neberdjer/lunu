use lunu_core::consts::auth::{MFA_RECOVERY_CODE_COUNT, TOTP_STEP_SECONDS};
use lunu_core::crypto::totp;
use lunu_core::models::MfaMethod;
use lunu_core::repo::UserMfaRepo;
use lunu_core::services::{LoginOutcome, MfaEnrollment};

use super::builders::*;
use super::*;

async fn confirmed_totp(db: &Db) -> (AuthService, lunu_core::models::User, Vec<String>) {
	let auth = admin_auth(db).await;
	let user = admin(db).await;
	let enrollment = auth
		.mfa_begin_enrollment(&user, MfaMethod::Totp)
		.await
		.unwrap();
	let recovery = auth
		.mfa_confirm_enrollment(&user, &current_totp(&enrollment))
		.await
		.unwrap();
	(auth, user, recovery)
}

async fn login_ticket(auth: &AuthService) -> String {
	match auth.login("admin", "password123").await.unwrap() {
		LoginOutcome::MfaRequired(challenge) => challenge.ticket,
		LoginOutcome::Authenticated(_) => panic!("a confirmed second factor must gate login"),
	}
}

async fn admin_auth(db: &Db) -> AuthService {
	let auth = auth_service(db);
	auth.setup_first_admin(
		"admin",
		"password123",
		Some("admin@example.com".to_string()),
	)
	.await
	.unwrap();
	auth
}

async fn admin(db: &Db) -> lunu_core::models::User {
	SqlxUserRepo::new(db.clone())
		.find_by_username("admin")
		.await
		.unwrap()
		.unwrap()
}

fn current_totp(enrollment: &MfaEnrollment) -> String {
	let secret = enrollment.secret.as_deref().unwrap();
	totp::totp_code(secret, now_seconds()).unwrap()
}

fn now_seconds() -> u64 {
	Utc::now().timestamp().max(0) as u64
}

#[tokio::test]
async fn totp_enrollment_requires_a_valid_code_and_then_gates_login() {
	let db = memory_db().await;
	let auth = admin_auth(&db).await;
	let user = admin(&db).await;

	let enrollment = auth
		.mfa_begin_enrollment(&user, MfaMethod::Totp)
		.await
		.unwrap();
	assert!(
		enrollment
			.otpauth_uri
			.as_deref()
			.unwrap()
			.starts_with("otpauth://totp/")
	);

	assert!(auth.mfa_confirm_enrollment(&user, "000000").await.is_err());
	assert!(!auth.mfa_status(&user).await.unwrap().enabled);

	auth.mfa_confirm_enrollment(&user, &current_totp(&enrollment))
		.await
		.unwrap();
	let status = auth.mfa_status(&user).await.unwrap();
	assert!(status.enabled);
	assert_eq!(status.method, Some(MfaMethod::Totp));

	let outcome = auth.login("admin", "password123").await.unwrap();
	let ticket = match outcome {
		LoginOutcome::MfaRequired(challenge) => {
			assert_eq!(challenge.method, MfaMethod::Totp);
			challenge.ticket
		}
		LoginOutcome::Authenticated(_) => panic!("a confirmed second factor must gate login"),
	};

	assert!(auth.mfa_verify(&ticket, "000000").await.is_err());
	let stored = SqlxUserMfaRepo::new(db.clone())
		.find_for_user(&user.id)
		.await
		.unwrap()
		.unwrap()
		.secret
		.unwrap();
	assert_ne!(
		stored,
		enrollment.secret.clone().unwrap(),
		"the totp secret must be encrypted at rest, not stored as the plaintext base32"
	);
	let code = totp::totp_code(enrollment.secret.as_deref().unwrap(), now_seconds()).unwrap();
	let authenticated = auth.mfa_verify(&ticket, &code).await.unwrap();
	assert_eq!(authenticated.user.id, user.id);
}

#[tokio::test]
async fn a_used_ticket_cannot_be_replayed() {
	let db = memory_db().await;
	let auth = admin_auth(&db).await;
	let user = admin(&db).await;
	let enrollment = auth
		.mfa_begin_enrollment(&user, MfaMethod::Totp)
		.await
		.unwrap();
	auth.mfa_confirm_enrollment(&user, &current_totp(&enrollment))
		.await
		.unwrap();

	let LoginOutcome::MfaRequired(challenge) = auth.login("admin", "password123").await.unwrap()
	else {
		panic!("expected a challenge");
	};
	let secret = enrollment.secret.unwrap();
	let code = totp::totp_code(&secret, now_seconds()).unwrap();
	auth.mfa_verify(&challenge.ticket, &code).await.unwrap();

	assert!(
		auth.mfa_verify(&challenge.ticket, &code).await.is_err(),
		"a ticket is single use"
	);
}

#[tokio::test]
async fn email_enrollment_and_login_delivers_and_checks_a_code() {
	let db = memory_db().await;
	let mailer = Arc::new(RecordingMailer::default());
	let auth = auth_service_with_mailer(&db, mailer.clone());
	auth.setup_first_admin(
		"admin",
		"password123",
		Some("admin@example.com".to_string()),
	)
	.await
	.unwrap();
	let user = admin(&db).await;

	auth.mfa_begin_enrollment(&user, MfaMethod::Email)
		.await
		.unwrap();
	auth.mfa_send_enrollment_code(&user, None).await.unwrap();
	assert_eq!(mailer.count(), 1, "the enrollment code is emailed");
}

#[tokio::test]
async fn email_enrollment_requires_a_verified_email() {
	let db = memory_db().await;
	let auth = auth_service(&db);
	auth.setup_first_admin("admin", "password123", None)
		.await
		.unwrap();
	let user = admin(&db).await;

	let Err(error) = auth.mfa_begin_enrollment(&user, MfaMethod::Email).await else {
		panic!("email 2fa needs a verified email");
	};
	assert!(matches!(error, Error::Validation(reason) if reason == "mfa-email-required"));
}

#[tokio::test]
async fn a_login_without_a_second_factor_still_returns_a_session() {
	let db = memory_db().await;
	let auth = admin_auth(&db).await;

	assert!(matches!(
		auth.login("admin", "password123").await.unwrap(),
		LoginOutcome::Authenticated(_)
	));
}

#[tokio::test]
async fn disabling_the_factor_reopens_direct_login() {
	let db = memory_db().await;
	let auth = admin_auth(&db).await;
	let user = admin(&db).await;
	let enrollment = auth
		.mfa_begin_enrollment(&user, MfaMethod::Totp)
		.await
		.unwrap();
	auth.mfa_confirm_enrollment(&user, &current_totp(&enrollment))
		.await
		.unwrap();

	auth.mfa_disable(&user).await.unwrap();
	assert!(!auth.mfa_status(&user).await.unwrap().enabled);
	assert!(matches!(
		auth.login("admin", "password123").await.unwrap(),
		LoginOutcome::Authenticated(_)
	));

	let _ = TOTP_STEP_SECONDS;
}

#[tokio::test]
async fn confirming_a_factor_issues_the_full_set_of_recovery_codes() {
	let db = memory_db().await;
	let (auth, user, recovery) = confirmed_totp(&db).await;

	assert_eq!(recovery.len(), MFA_RECOVERY_CODE_COUNT);
	assert_eq!(
		recovery
			.iter()
			.collect::<std::collections::HashSet<_>>()
			.len(),
		MFA_RECOVERY_CODE_COUNT,
		"every issued recovery code is distinct"
	);
	let status = auth.mfa_status(&user).await.unwrap();
	assert_eq!(
		status.recovery_codes_remaining,
		Some(MFA_RECOVERY_CODE_COUNT as i64)
	);
}

#[tokio::test]
async fn a_recovery_code_stands_in_for_a_lost_authenticator_and_is_single_use() {
	let db = memory_db().await;
	let (auth, user, recovery) = confirmed_totp(&db).await;
	let code = recovery[0].clone();

	let ticket = login_ticket(&auth).await;
	let authenticated = auth.mfa_verify(&ticket, &code).await.unwrap();
	assert_eq!(authenticated.user.id, user.id);
	assert_eq!(
		auth.mfa_status(&user)
			.await
			.unwrap()
			.recovery_codes_remaining,
		Some(MFA_RECOVERY_CODE_COUNT as i64 - 1),
		"a redeemed recovery code is spent"
	);

	let ticket = login_ticket(&auth).await;
	assert!(
		auth.mfa_verify(&ticket, &code).await.is_err(),
		"the same recovery code cannot be redeemed twice"
	);
}

#[tokio::test]
async fn regenerating_recovery_codes_retires_the_previous_set() {
	let db = memory_db().await;
	let (auth, user, old) = confirmed_totp(&db).await;

	let fresh = auth.mfa_regenerate_recovery_codes(&user).await.unwrap();
	assert_eq!(fresh.len(), MFA_RECOVERY_CODE_COUNT);
	assert!(
		fresh.iter().all(|code| !old.contains(code)),
		"regeneration must not re-issue an old code"
	);
	assert_eq!(
		auth.mfa_status(&user)
			.await
			.unwrap()
			.recovery_codes_remaining,
		Some(MFA_RECOVERY_CODE_COUNT as i64),
		"the counter reflects the fresh set, not the sum of both"
	);

	let ticket = login_ticket(&auth).await;
	assert!(
		auth.mfa_verify(&ticket, &old[0]).await.is_err(),
		"a code from the retired set no longer logs in"
	);
	let ticket = login_ticket(&auth).await;
	auth.mfa_verify(&ticket, &fresh[0]).await.unwrap();
}

#[tokio::test]
async fn disabling_the_factor_clears_its_recovery_codes() {
	let db = memory_db().await;
	let (auth, user, recovery) = confirmed_totp(&db).await;

	auth.mfa_disable(&user).await.unwrap();
	let status = auth.mfa_status(&user).await.unwrap();
	assert!(!status.enabled);
	assert_eq!(
		status.recovery_codes_remaining, None,
		"a disabled factor reports no recovery codes"
	);

	let enrollment = auth
		.mfa_begin_enrollment(&user, MfaMethod::Totp)
		.await
		.unwrap();
	auth.mfa_confirm_enrollment(&user, &current_totp(&enrollment))
		.await
		.unwrap();
	let ticket = login_ticket(&auth).await;
	assert!(
		auth.mfa_verify(&ticket, &recovery[0]).await.is_err(),
		"a recovery code from a disabled enrollment must not survive re-enrollment"
	);
}

#[tokio::test]
async fn an_admin_reset_unlocks_a_user_and_rejects_an_unknown_id() {
	let db = memory_db().await;
	let (auth, user, _) = confirmed_totp(&db).await;

	auth.mfa_admin_reset(&user.id).await.unwrap();
	assert!(!auth.mfa_status(&user).await.unwrap().enabled);
	assert!(
		matches!(
			auth.login("admin", "password123").await.unwrap(),
			LoginOutcome::Authenticated(_)
		),
		"after a reset the user logs in with the password alone"
	);

	assert!(matches!(
		auth.mfa_admin_reset("does-not-exist").await,
		Err(Error::NotFound(_))
	));
}
