use super::*;

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

#[tokio::test]
async fn a_totp_code_cannot_be_replayed_on_a_second_login() {
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
	let secret = enrollment.secret.as_deref().unwrap();
	let code = totp::totp_code(secret, now_seconds()).unwrap();

	let first = match auth.login("admin", "password123").await.unwrap() {
		LoginOutcome::MfaRequired(challenge) => challenge.ticket,
		LoginOutcome::Authenticated(_) => panic!("mfa must gate login"),
	};
	auth.mfa_verify(&first, &code).await.unwrap();

	let second = match auth.login("admin", "password123").await.unwrap() {
		LoginOutcome::MfaRequired(challenge) => challenge.ticket,
		LoginOutcome::Authenticated(_) => panic!("mfa must gate login"),
	};
	assert!(
		auth.mfa_verify(&second, &code).await.is_err(),
		"a captured code replayed within its window must be rejected once it has been spent"
	);
}
