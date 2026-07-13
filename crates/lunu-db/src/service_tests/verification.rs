use lunu_core::consts::settings::REQUIRE_EMAIL_VERIFICATION;
use lunu_core::services::{InviteService, Registration};

use super::builders::*;
use super::*;

async fn enable_verification(db: &Db) {
	settings_service(db)
		.set(REQUIRE_EMAIL_VERIFICATION, "on")
		.await
		.unwrap();
}

#[tokio::test]
async fn gate_blocks_login_until_verified() {
	let db = memory_db().await;
	enable_verification(&db).await;
	let auth = auth_service(&db);
	auth.setup_first_admin("admin", "password123", None)
		.await
		.unwrap();
	let user = user_service(&db)
		.create(
			"bob",
			"hunter2password",
			Some("bob@example.com".to_string()),
			Role::User,
		)
		.await
		.unwrap();

	assert!(matches!(
		auth.login("bob", "hunter2password").await,
		Err(Error::Validation(_))
	));

	seed_verification_code(&db, &user.id, "424242").await;
	auth.verify_email("bob@example.com", "424242")
		.await
		.unwrap();

	assert!(auth.login("bob", "hunter2password").await.is_ok());
}

#[tokio::test]
async fn wrong_code_burns_attempts_and_login_stays_blocked() {
	let db = memory_db().await;
	enable_verification(&db).await;
	let auth = auth_service(&db);
	auth.setup_first_admin("admin", "password123", None)
		.await
		.unwrap();
	let user = user_service(&db)
		.create(
			"bob",
			"hunter2password",
			Some("bob@example.com".to_string()),
			Role::User,
		)
		.await
		.unwrap();
	seed_verification_code(&db, &user.id, "424242").await;

	for _ in 0..5 {
		assert!(
			auth.verify_email("bob@example.com", "000000")
				.await
				.is_err()
		);
	}

	assert!(
		auth.verify_email("bob@example.com", "424242")
			.await
			.is_err()
	);
	assert!(matches!(
		auth.login("bob", "hunter2password").await,
		Err(Error::Validation(_))
	));
}

#[tokio::test]
async fn register_with_gate_pends_and_sends_welcome_and_verification() {
	let db = memory_db().await;
	enable_verification(&db).await;
	let mailer = Arc::new(RecordingMailer::default());
	let auth = auth_service_with_mailer(&db, mailer.clone());
	let admin = auth
		.setup_first_admin("admin", "password123", None)
		.await
		.unwrap();
	let invites = InviteService::new(Arc::new(SqlxInviteRepo::new(db.clone())));
	let issued = invites
		.create(
			&admin.user.id,
			Role::User,
			Some("bob@example.com".to_string()),
			1,
			None,
		)
		.await
		.unwrap();

	let outcome = auth
		.register_with_invite(&issued.code, "bob", "hunter2password", None)
		.await
		.unwrap();

	assert!(matches!(outcome, Registration::PendingVerification));
	assert_eq!(mailer.count(), 2);
}

#[tokio::test]
async fn disabled_gate_registers_active_with_welcome_only() {
	let db = memory_db().await;
	let mailer = Arc::new(RecordingMailer::default());
	let auth = auth_service_with_mailer(&db, mailer.clone());
	let admin = auth
		.setup_first_admin("admin", "password123", None)
		.await
		.unwrap();
	let invites = InviteService::new(Arc::new(SqlxInviteRepo::new(db.clone())));
	let issued = invites
		.create(
			&admin.user.id,
			Role::User,
			Some("bob@example.com".to_string()),
			1,
			None,
		)
		.await
		.unwrap();

	let outcome = auth
		.register_with_invite(&issued.code, "bob", "hunter2password", None)
		.await
		.unwrap();

	assert!(matches!(outcome, Registration::Active(_)));
	assert_eq!(mailer.count(), 1);
}
