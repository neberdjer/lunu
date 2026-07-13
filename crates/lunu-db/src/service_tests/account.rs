use super::builders::*;
use super::*;

#[tokio::test]
async fn update_email_validates_and_normalizes() {
	let db = memory_db().await;
	let auth = auth_service(&db);
	let admin = auth
		.setup_first_admin("admin", "password123", None)
		.await
		.unwrap();
	let users = UserService::new(
		Arc::new(SqlxUserRepo::new(db.clone())),
		Arc::new(SqlxSessionRepo::new(db.clone())),
		Arc::new(SqlxUserSettingsRepo::new(db.clone())),
	);

	assert!(matches!(
		users
			.update_profile(&admin.user.id, Some("not-an-email".to_string()), None, None)
			.await,
		Err(Error::Validation(_))
	));

	let updated = users
		.update_profile(
			&admin.user.id,
			Some("  me@example.com  ".to_string()),
			None,
			Some("en".to_string()),
		)
		.await
		.unwrap();
	assert_eq!(updated.email.as_deref(), Some("me@example.com"));
	assert_eq!(updated.locale.as_deref(), Some("en-US"));

	let cleared = users
		.update_profile(&admin.user.id, None, None, None)
		.await
		.unwrap();
	assert_eq!(cleared.email, None);
	assert_eq!(cleared.locale, None);
}

#[tokio::test]
async fn settings_reject_unknown_key_and_invalid_values() {
	let db = memory_db().await;
	let settings = settings_service(&db);

	assert!(matches!(
		settings.set("not_a_real_setting", "x").await,
		Err(Error::Validation(_))
	));
	assert!(matches!(
		settings.set("prowlarr_url", "localhost").await,
		Err(Error::Validation(_))
	));
	assert!(matches!(
		settings.set("metadata_region", "zz").await,
		Err(Error::Validation(_))
	));
}

#[tokio::test]
async fn settings_derive_secret_flag_from_registry() {
	let db = memory_db().await;
	let settings = settings_service(&db);

	settings.set("prowlarr_api_key", "topsecret").await.unwrap();

	let stored = SqlxSettingsRepo::new(db.clone())
		.get("prowlarr_api_key")
		.await
		.unwrap()
		.unwrap();
	assert!(stored.encrypted);
	assert_ne!(stored.value, "topsecret");
	assert_eq!(
		settings.get("prowlarr_api_key").await.unwrap().as_deref(),
		Some("topsecret")
	);
}

#[tokio::test]
async fn settings_encrypts_secret_values() {
	let db = memory_db().await;
	let settings = settings_service(&db);

	settings
		.set("qbittorrent_password", "s3cret")
		.await
		.unwrap();
	settings.set("download_dir", "/data").await.unwrap();

	assert_eq!(
		settings
			.get("qbittorrent_password")
			.await
			.unwrap()
			.as_deref(),
		Some("s3cret")
	);
	assert_eq!(
		settings.get("download_dir").await.unwrap().as_deref(),
		Some("/data")
	);

	let stored = SqlxSettingsRepo::new(db.clone())
		.get("qbittorrent_password")
		.await
		.unwrap()
		.unwrap();
	assert!(stored.encrypted);
	assert_ne!(stored.value, "s3cret");
}

#[tokio::test]
async fn settings_view_masks_secrets() {
	let db = memory_db().await;
	let settings = settings_service(&db);

	settings
		.set("qbittorrent_password", "s3cret")
		.await
		.unwrap();
	settings.set("download_dir", "/data").await.unwrap();

	let secret = settings
		.view("qbittorrent_password")
		.await
		.unwrap()
		.unwrap();
	assert!(secret.secret);
	assert!(secret.value.is_none());

	let plain = settings.view("download_dir").await.unwrap().unwrap();
	assert!(!plain.secret);
	assert_eq!(plain.value.as_deref(), Some("/data"));

	assert!(settings.view("missing").await.unwrap().is_none());
}

#[tokio::test]
async fn api_key_issue_verify_revoke() {
	let db = memory_db().await;
	let keys = ApiKeyService::new(Arc::new(SqlxApiKeyRepo::new(db.clone())));

	let issued = keys
		.issue("user-1", "cli", vec!["admin".to_string()], None)
		.await
		.unwrap();

	assert!(matches!(
		keys.issue("user-1", "bad", vec!["read".to_string()], None)
			.await,
		Err(Error::Validation(_))
	));

	let verified = keys.verify(&issued.secret).await.unwrap().unwrap();
	assert_eq!(verified.user_id, "user-1");

	assert!(
		keys.revoke_for_user("someone-else", &issued.api_key.id)
			.await
			.is_err()
	);

	keys.revoke_for_user("user-1", &issued.api_key.id)
		.await
		.unwrap();
	assert!(keys.verify(&issued.secret).await.unwrap().is_none());
}

#[tokio::test]
async fn duplicate_username_is_conflict_not_db_error() {
	let db = memory_db().await;
	let repo = SqlxUserRepo::new(db.clone());

	let now = Utc::now();
	let user = |id: &str| User {
		id: id.to_string(),
		username: "alice".to_string(),
		email: None,
		password_hash: Some("hash".to_string()),
		role: Role::User,
		auth_source: AuthSource::Local,
		display_name: None,
		locale: None,
		enabled: true,
		email_verified: true,
		created_at: now,
		updated_at: now,
	};

	repo.create(&user("u1")).await.unwrap();
	match repo.create(&user("u2")).await {
		Err(Error::Conflict(_)) => {}
		other => panic!("expected Conflict on duplicate username, got {other:?}"),
	}
}

#[tokio::test]
async fn user_settings_upsert() {
	let db = memory_db().await;
	let settings = SqlxUserSettingsRepo::new(db.clone());

	assert!(settings.get("u1").await.unwrap().is_none());

	settings
		.upsert(&UserSettings {
			user_id: "u1".to_string(),
			auto_approve: true,
			request_quota: Some(5),
			quota_days: Some(7),
			updated_at: Utc::now(),
		})
		.await
		.unwrap();

	let loaded = settings.get("u1").await.unwrap().unwrap();
	assert!(loaded.auto_approve);
	assert_eq!(loaded.request_quota, Some(5));
	assert_eq!(loaded.quota_days, Some(7));
}
