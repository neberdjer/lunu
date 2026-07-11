use std::sync::Arc;

use chrono::Utc;
use lunu_core::consts::crypto::SETTINGS_ENCRYPTION_CONTEXT;
use lunu_core::crypto::Encryptor;
use lunu_core::models::{MetadataCacheEntry, Request, RequestStatus, Role, UserSettings};
use lunu_core::repo::{MetadataCacheRepo, RequestRepo, SettingsRepo, UserSettingsRepo};
use lunu_core::services::{ApiKeyService, AuthService, InviteService, SettingsService};
use sqlx::any::{AnyPoolOptions, install_default_drivers};

use crate::repos::{
	SqlxApiKeyRepo, SqlxInviteRepo, SqlxMetadataCacheRepo, SqlxRequestRepo, SqlxSessionRepo,
	SqlxSettingsRepo, SqlxUserRepo, SqlxUserSettingsRepo,
};
use crate::{Db, run_migrations};

async fn memory_db() -> Db {
	install_default_drivers();
	let db = AnyPoolOptions::new()
		.max_connections(1)
		.connect("sqlite::memory:")
		.await
		.unwrap();
	run_migrations(&db).await.unwrap();
	db
}

fn auth_service(db: &Db) -> AuthService {
	AuthService::new(
		Arc::new(SqlxUserRepo::new(db.clone())),
		Arc::new(SqlxSessionRepo::new(db.clone())),
		Arc::new(SqlxInviteRepo::new(db.clone())),
	)
}

#[tokio::test]
async fn auth_setup_login_validate_logout() {
	let db = memory_db().await;
	let auth = auth_service(&db);

	assert!(auth.needs_setup().await.unwrap());
	let admin = auth
		.setup_first_admin("admin", "password123", None)
		.await
		.unwrap();
	assert_eq!(admin.user.role, Role::Admin);
	assert!(!auth.needs_setup().await.unwrap());
	assert!(auth.setup_first_admin("x", "y", None).await.is_err());

	assert!(auth.login("admin", "wrong").await.is_err());
	let authed = auth.login("admin", "password123").await.unwrap();
	assert_eq!(authed.user.username, "admin");

	let validated = auth
		.validate_session(&authed.session_token)
		.await
		.unwrap()
		.unwrap();
	assert_eq!(validated.id, admin.user.id);

	auth.logout(&authed.session_token).await.unwrap();
	assert!(
		auth.validate_session(&authed.session_token)
			.await
			.unwrap()
			.is_none()
	);
}

#[tokio::test]
async fn register_with_invite_then_exhausted() {
	let db = memory_db().await;
	let auth = auth_service(&db);
	let invites = InviteService::new(Arc::new(SqlxInviteRepo::new(db.clone())));

	let admin = auth
		.setup_first_admin("admin", "password123", None)
		.await
		.unwrap();
	let issued = invites
		.create(&admin.user.id, Role::User, None, 1, None)
		.await
		.unwrap();

	let registered = auth
		.register_with_invite(&issued.code, "bob", "hunter2password")
		.await
		.unwrap();
	assert_eq!(registered.user.username, "bob");
	assert_eq!(registered.user.role, Role::User);

	assert!(
		auth.register_with_invite(&issued.code, "carol", "password123")
			.await
			.is_err()
	);
}

#[tokio::test]
async fn settings_encrypts_secret_values() {
	let db = memory_db().await;
	let encryptor = Encryptor::new("dev-master-key-value", SETTINGS_ENCRYPTION_CONTEXT).unwrap();
	let settings = SettingsService::new(Arc::new(SqlxSettingsRepo::new(db.clone())), encryptor);

	settings
		.set("qbittorrent_password", "s3cret", true)
		.await
		.unwrap();
	settings.set("download_dir", "/data", false).await.unwrap();

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
async fn api_key_issue_verify_revoke() {
	let db = memory_db().await;
	let keys = ApiKeyService::new(Arc::new(SqlxApiKeyRepo::new(db.clone())));

	let issued = keys
		.issue("user-1", "cli", vec!["read".to_string()], None)
		.await
		.unwrap();

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
async fn metadata_cache_put_get_upsert() {
	let db = memory_db().await;
	let cache = SqlxMetadataCacheRepo::new(db.clone());

	assert!(
		cache
			.get("audnexus", "book", "B01")
			.await
			.unwrap()
			.is_none()
	);

	let entry = |payload: &str| MetadataCacheEntry {
		provider: "audnexus".to_string(),
		kind: "book".to_string(),
		key: "B01".to_string(),
		payload: payload.to_string(),
		fetched_at: Utc::now(),
	};

	cache.put(&entry("{\"v\":1}")).await.unwrap();
	assert_eq!(
		cache
			.get("audnexus", "book", "B01")
			.await
			.unwrap()
			.unwrap()
			.payload,
		"{\"v\":1}"
	);

	cache.put(&entry("{\"v\":2}")).await.unwrap();
	assert_eq!(
		cache
			.get("audnexus", "book", "B01")
			.await
			.unwrap()
			.unwrap()
			.payload,
		"{\"v\":2}"
	);
}

#[tokio::test]
async fn request_lifecycle_and_quota_count() {
	let db = memory_db().await;
	let requests = SqlxRequestRepo::new(db.clone());

	let now = Utc::now();
	let request = Request {
		id: "r1".to_string(),
		user_id: "u1".to_string(),
		asin: "B01".to_string(),
		title: "The Hobbit".to_string(),
		author: Some("Tolkien".to_string()),
		cover_url: None,
		status: RequestStatus::Pending,
		approved_by: None,
		created_at: now,
		updated_at: now,
	};
	requests.create(&request).await.unwrap();

	let found = requests
		.find_by_user_and_asin("u1", "B01")
		.await
		.unwrap()
		.unwrap();
	assert_eq!(found.status, RequestStatus::Pending);
	assert_eq!(found.author.as_deref(), Some("Tolkien"));

	let mut approved = found;
	approved.status = RequestStatus::Approved;
	approved.approved_by = Some("admin".to_string());
	requests.update(&approved).await.unwrap();
	assert_eq!(
		requests.find_by_id("r1").await.unwrap().unwrap().status,
		RequestStatus::Approved
	);

	assert_eq!(
		requests
			.count_for_user_since("u1", now - chrono::Duration::days(1))
			.await
			.unwrap(),
		1
	);
	assert_eq!(requests.list_for_user("u1").await.unwrap().len(), 1);
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
