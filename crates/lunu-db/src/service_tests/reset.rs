use super::builders::*;
use super::*;

#[tokio::test]
async fn password_reset_rotates_password_and_consumes_code() {
	let db = memory_db().await;
	let auth = auth_service(&db);
	let admin = auth
		.setup_first_admin(
			"admin",
			"password123",
			Some("admin@example.com".to_string()),
		)
		.await
		.unwrap();

	seed_reset_code(&db, &admin.user.id, "123456").await;

	auth.reset_password("admin@example.com", "123456", "brandnewpass")
		.await
		.unwrap();

	assert!(auth.login("admin", "brandnewpass").await.is_ok());
	assert!(auth.login("admin", "password123").await.is_err());
	assert!(
		auth.reset_password("admin@example.com", "123456", "anotherpass")
			.await
			.is_err()
	);
}

#[tokio::test]
async fn request_password_reset_issues_for_known_local_email_and_cools_down() {
	let db = memory_db().await;
	let auth = auth_service(&db);
	auth.setup_first_admin(
		"admin",
		"password123",
		Some("admin@example.com".to_string()),
	)
	.await
	.unwrap();

	auth.request_password_reset("admin@example.com", None)
		.await
		.unwrap();
	auth.request_password_reset("admin@example.com", None)
		.await
		.unwrap();
	auth.request_password_reset("nobody@example.com", None)
		.await
		.unwrap();

	assert_eq!(reset_token_count(&db).await, 1);
}

#[tokio::test]
async fn wrong_reset_code_locks_after_max_attempts() {
	let db = memory_db().await;
	let auth = auth_service(&db);
	let admin = auth
		.setup_first_admin(
			"admin",
			"password123",
			Some("admin@example.com".to_string()),
		)
		.await
		.unwrap();

	seed_reset_code(&db, &admin.user.id, "123456").await;

	for _ in 0..5 {
		assert!(
			auth.reset_password("admin@example.com", "000000", "brandnewpass")
				.await
				.is_err()
		);
	}

	assert_eq!(reset_token_count(&db).await, 1);
	assert!(
		auth.reset_password("admin@example.com", "123456", "brandnewpass")
			.await
			.is_err()
	);
	assert!(auth.login("admin", "password123").await.is_ok());
}

#[tokio::test]
async fn exhausted_reset_code_cannot_be_reissued_within_cooldown() {
	let db = memory_db().await;
	let auth = auth_service(&db);
	let admin = auth
		.setup_first_admin(
			"admin",
			"password123",
			Some("admin@example.com".to_string()),
		)
		.await
		.unwrap();

	seed_reset_code(&db, &admin.user.id, "123456").await;

	for _ in 0..5 {
		assert!(
			auth.reset_password("admin@example.com", "000000", "brandnewpass")
				.await
				.is_err()
		);
	}

	auth.request_password_reset("admin@example.com", None)
		.await
		.unwrap();

	assert_eq!(reset_token_count(&db).await, 1);
	assert!(
		auth.reset_password("admin@example.com", "123456", "brandnewpass")
			.await
			.is_err()
	);
}

#[tokio::test]
async fn new_device_login_alerts_only_for_unseen_device() {
	let db = memory_db().await;
	let mailer = Arc::new(RecordingMailer::default());
	let auth = auth_service_with_mailer(&db, mailer.clone());
	let admin = auth
		.setup_first_admin(
			"admin",
			"password123",
			Some("admin@example.com".to_string()),
		)
		.await
		.unwrap();

	auth.record_login_device(&admin.user, &admin.session_id, Some("Firefox"), None)
		.await
		.unwrap();
	assert_eq!(mailer.count(), 0);

	let second = auth.login("admin", "password123").await.unwrap();
	auth.record_login_device(&second.user, &second.session_id, Some("Firefox"), None)
		.await
		.unwrap();
	assert_eq!(mailer.count(), 0);

	let third = auth.login("admin", "password123").await.unwrap();
	auth.record_login_device(&third.user, &third.session_id, Some("Chrome"), None)
		.await
		.unwrap();
	assert_eq!(mailer.count(), 1);
}
