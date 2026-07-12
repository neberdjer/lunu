use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use lunu_core::consts::crypto::SETTINGS_ENCRYPTION_CONTEXT;
use lunu_core::consts::download::MONITOR_MAX_MISSES;
use lunu_core::crypto::Encryptor;
use lunu_core::models::{
	Activity, AuthSource, Book, Chapters, Download, DownloadState, DownloadStatus, Job, JobStatus,
	JobType, MetadataCacheEntry, MonitorPayload, NotificationEvent, NotificationKind, Protocol,
	QualityProfile, Release, Request, RequestStatus, Role, User, UserSettings,
};
use lunu_core::repo::{
	ActivityRepo, DownloadRepo, JobRepo, MetadataCacheRepo, QualityProfileRepo, RequestRepo,
	SettingsRepo, UserRepo, UserSettingsRepo,
};
use lunu_core::services::{
	ActivityService, ApiKeyService, AuthService, ImportService, InviteService, JobService,
	MediaService, MetadataService, MonitorService, NotificationInboxService, NotificationService,
	ReleaseService, RequestService, SettingsService, UserService,
};
use lunu_core::traits::{
	AuthProvider, DownloadClient, EventPublisher, ExternalIdentity, Importer, Indexer,
	MetadataProvider, Notifier,
};
use lunu_core::{Error, Result as CoreResult};
use sqlx::any::{AnyPoolOptions, install_default_drivers};

use crate::repos::{
	SqlxActivityRepo, SqlxApiKeyRepo, SqlxBlocklistRepo, SqlxDownloadRepo, SqlxInviteRepo,
	SqlxJobRepo, SqlxMediaRepo, SqlxMetadataCacheRepo, SqlxQualityProfileRepo, SqlxRequestRepo,
	SqlxSessionRepo, SqlxSettingsRepo, SqlxUserNotificationRepo, SqlxUserRepo,
	SqlxUserSettingsRepo,
};
use crate::{Db, run_migrations};

static SCHEMA_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

async fn memory_db() -> Db {
	install_default_drivers();
	match std::env::var("LUNU_TEST_DATABASE_URL") {
		Ok(url) => postgres_isolated_db(&url).await,
		Err(_) => {
			let db = AnyPoolOptions::new()
				.max_connections(1)
				.connect("sqlite::memory:")
				.await
				.unwrap();
			run_migrations(&db).await.unwrap();
			db
		}
	}
}

async fn postgres_isolated_db(url: &str) -> Db {
	use std::sync::atomic::Ordering;

	let schema = format!(
		"lunu_test_{}_{}",
		std::process::id(),
		SCHEMA_SEQ.fetch_add(1, Ordering::Relaxed)
	);

	let admin = AnyPoolOptions::new()
		.max_connections(1)
		.connect(url)
		.await
		.unwrap();
	sqlx::query(&format!("DROP SCHEMA IF EXISTS {schema} CASCADE"))
		.execute(&admin)
		.await
		.unwrap();
	sqlx::query(&format!("CREATE SCHEMA {schema}"))
		.execute(&admin)
		.await
		.unwrap();
	admin.close().await;

	let db = AnyPoolOptions::new()
		.max_connections(1)
		.after_connect(move |conn, _meta| {
			let schema = schema.clone();
			Box::pin(async move {
				sqlx::query(&format!("SET search_path TO {schema}"))
					.execute(conn)
					.await?;
				Ok(())
			})
		})
		.connect(url)
		.await
		.unwrap();
	run_migrations(&db).await.unwrap();
	db
}

fn has_question_mark_in_string_literal(src: &str) -> bool {
	let mut in_string = false;
	let mut escaped = false;
	for c in src.chars() {
		if in_string {
			if escaped {
				escaped = false;
			} else if c == '\\' {
				escaped = true;
			} else if c == '"' {
				in_string = false;
			} else if c == '?' {
				return true;
			}
		} else if c == '"' {
			in_string = true;
		}
	}
	false
}

#[test]
fn placeholder_detector_flags_strings_not_operators() {
	assert!(has_question_mark_in_string_literal(
		"sqlx::query(\"SELECT * FROM t WHERE id = ?\")"
	));
	assert!(!has_question_mark_in_string_literal(
		"sqlx::query(\"SELECT * FROM t WHERE id = $1\")"
	));
	assert!(!has_question_mark_in_string_literal(
		"row.try_get(\"id\").map_err(db_error)?;"
	));
}

#[test]
fn repo_sql_uses_numbered_placeholders_not_question_marks() {
	let repos = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/repos");
	let mut offenders: Vec<String> = Vec::new();

	for entry in std::fs::read_dir(&repos).unwrap() {
		let path = entry.unwrap().path();
		if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
			continue;
		}
		let source = std::fs::read_to_string(&path).unwrap();
		if has_question_mark_in_string_literal(&source) {
			offenders.push(path.file_name().unwrap().to_string_lossy().into_owned());
		}
	}

	assert!(
		offenders.is_empty(),
		"bare positional placeholders found in repo SQL; use numbered $1..$N for SQLite and Postgres portability: {offenders:?}"
	);
}

#[tokio::test]
async fn numbered_placeholders_roundtrip_on_sqlite() {
	use sqlx::Row;

	let db = memory_db().await;
	sqlx::query("CREATE TABLE ph (a TEXT, b BIGINT)")
		.execute(&db)
		.await
		.unwrap();
	sqlx::query("INSERT INTO ph (a, b) VALUES ($1, $2)")
		.bind("x")
		.bind(7_i64)
		.execute(&db)
		.await
		.unwrap();
	let row = sqlx::query("SELECT a, b FROM ph WHERE a = $1 ORDER BY b LIMIT $2")
		.bind("x")
		.bind(1_i64)
		.fetch_one(&db)
		.await
		.unwrap();
	let a: String = row.try_get("a").unwrap();
	let b: i64 = row.try_get("b").unwrap();
	assert_eq!(a, "x");
	assert_eq!(b, 7);
}

#[tokio::test]
async fn migrations_apply_and_every_table_is_queryable() {
	let db = memory_db().await;

	let tables = [
		"users",
		"sessions",
		"api_keys",
		"invites",
		"settings",
		"metadata_cache",
		"requests",
		"user_settings",
		"quality_profiles",
		"downloads",
		"jobs",
		"activity",
		"blocklist",
	];

	for table in tables {
		let query = format!("SELECT COUNT(*) FROM {table}");
		sqlx::query(&query)
			.fetch_one(&db)
			.await
			.unwrap_or_else(|error| panic!("table {table} is not queryable: {error}"));
	}
}

fn auth_service(db: &Db) -> AuthService {
	AuthService::new(
		Arc::new(SqlxUserRepo::new(db.clone())),
		Arc::new(SqlxSessionRepo::new(db.clone())),
		Arc::new(SqlxInviteRepo::new(db.clone())),
		None,
	)
}

fn auth_service_with_provider(db: &Db, provider: Arc<dyn AuthProvider>) -> AuthService {
	AuthService::new(
		Arc::new(SqlxUserRepo::new(db.clone())),
		Arc::new(SqlxSessionRepo::new(db.clone())),
		Arc::new(SqlxInviteRepo::new(db.clone())),
		Some(provider),
	)
}

struct FakeAuthProvider {
	username: String,
	password: String,
	identity: ExternalIdentity,
}

#[async_trait]
impl AuthProvider for FakeAuthProvider {
	fn name(&self) -> &'static str {
		"fake"
	}
	async fn authenticate(
		&self,
		username: &str,
		password: &str,
	) -> CoreResult<Option<ExternalIdentity>> {
		if username == self.username && password == self.password {
			Ok(Some(self.identity.clone()))
		} else {
			Ok(None)
		}
	}
}

#[tokio::test]
async fn abs_login_provisions_and_links_external_user() {
	let db = memory_db().await;
	let provider = Arc::new(FakeAuthProvider {
		username: "absuser".to_string(),
		password: "abspass".to_string(),
		identity: ExternalIdentity {
			username: "absuser".to_string(),
			email: Some("abs@example.com".to_string()),
		},
	});
	let auth = auth_service_with_provider(&db, provider);

	assert!(matches!(
		auth.login("absuser", "wrong").await,
		Err(Error::Unauthorized)
	));

	let first = auth.login("absuser", "abspass").await.unwrap();
	assert_eq!(first.user.auth_source, AuthSource::Abs);
	assert_eq!(first.user.email.as_deref(), Some("abs@example.com"));
	assert!(first.user.password_hash.is_none());
	assert!(
		auth.validate_session(&first.session_token)
			.await
			.unwrap()
			.is_some()
	);

	let second = auth.login("absuser", "abspass").await.unwrap();
	assert_eq!(second.user.id, first.user.id);
	assert_eq!(SqlxUserRepo::new(db.clone()).count().await.unwrap(), 1);
}

#[tokio::test]
async fn abs_credentials_cannot_unlock_local_account() {
	let db = memory_db().await;
	let users = UserService::new(
		Arc::new(SqlxUserRepo::new(db.clone())),
		Arc::new(SqlxSessionRepo::new(db.clone())),
		Arc::new(SqlxUserSettingsRepo::new(db.clone())),
	);
	users
		.create("alice", "password123", None, Role::User)
		.await
		.unwrap();

	let provider = Arc::new(FakeAuthProvider {
		username: "ghost".to_string(),
		password: "ghostpass".to_string(),
		identity: ExternalIdentity {
			username: "alice".to_string(),
			email: None,
		},
	});
	let auth = auth_service_with_provider(&db, provider);

	assert!(matches!(
		auth.login("ghost", "ghostpass").await,
		Err(Error::Unauthorized)
	));
	assert!(auth.login("alice", "password123").await.is_ok());
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
async fn change_password_rotates_sessions_and_rejects_wrong_current() {
	let db = memory_db().await;
	let auth = auth_service(&db);
	let admin = auth
		.setup_first_admin("admin", "password123", None)
		.await
		.unwrap();

	assert!(matches!(
		auth.change_password(&admin.user.id, "wrongcurrent", "newpassword123")
			.await,
		Err(Error::Unauthorized)
	));
	assert!(matches!(
		auth.change_password(&admin.user.id, "password123", "short")
			.await,
		Err(Error::Validation(_))
	));

	let rotated = auth
		.change_password(&admin.user.id, "password123", "newpassword123")
		.await
		.unwrap();

	assert!(
		auth.validate_session(&admin.session_token)
			.await
			.unwrap()
			.is_none()
	);
	assert!(
		auth.validate_session(&rotated.session_token)
			.await
			.unwrap()
			.is_some()
	);
	assert!(auth.login("admin", "newpassword123").await.is_ok());
	assert!(auth.login("admin", "password123").await.is_err());
}

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
			.update_profile(&admin.user.id, Some("not-an-email".to_string()), None)
			.await,
		Err(Error::Validation(_))
	));

	let updated = users
		.update_profile(&admin.user.id, Some("  me@example.com  ".to_string()), None)
		.await
		.unwrap();
	assert_eq!(updated.email.as_deref(), Some("me@example.com"));

	let cleared = users
		.update_profile(&admin.user.id, None, None)
		.await
		.unwrap();
	assert_eq!(cleared.email, None);
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

fn user_service(db: &Db) -> UserService {
	UserService::new(
		Arc::new(SqlxUserRepo::new(db.clone())),
		Arc::new(SqlxSessionRepo::new(db.clone())),
		Arc::new(SqlxUserSettingsRepo::new(db.clone())),
	)
}

#[tokio::test]
async fn cannot_disable_or_delete_last_admin() {
	let db = memory_db().await;
	let admin = auth_service(&db)
		.setup_first_admin("admin", "password123", None)
		.await
		.unwrap()
		.user;
	let users = user_service(&db);

	assert!(matches!(
		users.set_enabled(&admin.id, false).await,
		Err(Error::Conflict(_))
	));
	assert!(matches!(
		users.delete(&admin.id).await,
		Err(Error::Conflict(_))
	));

	let second = users
		.create("admin2", "password123", None, Role::Admin)
		.await
		.unwrap();
	users.delete(&admin.id).await.unwrap();
	assert!(matches!(
		users.delete(&second.id).await,
		Err(Error::Conflict(_))
	));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_admin_removal_never_drops_below_one() {
	let db = memory_db().await;
	let first = auth_service(&db)
		.setup_first_admin("admin1", "password123", None)
		.await
		.unwrap()
		.user;
	let users = Arc::new(user_service(&db));
	let second = users
		.create("admin2", "password123", None, Role::Admin)
		.await
		.unwrap();

	let a = users.clone();
	let b = users.clone();
	let first_id = first.id.clone();
	let second_id = second.id.clone();
	let t1 = tokio::spawn(async move { a.set_enabled(&first_id, false).await });
	let t2 = tokio::spawn(async move { b.delete(&second_id).await.map(|_| ()) });
	let r1 = t1.await.unwrap();
	let r2 = t2.await.unwrap();

	let failures = [r1.is_err(), r2.is_err()]
		.into_iter()
		.filter(|e| *e)
		.count();
	assert_eq!(
		failures, 1,
		"exactly one concurrent admin removal must be rejected"
	);
	assert!(
		SqlxUserRepo::new(db.clone())
			.count_enabled_admins_excluding("none")
			.await
			.unwrap() >= 1
	);
}

#[tokio::test]
async fn create_initial_admin_rejects_second_insert() {
	let db = memory_db().await;
	let repo = SqlxUserRepo::new(db.clone());
	let mk = |id: &str, name: &str| User {
		id: id.to_string(),
		username: name.to_string(),
		email: None,
		password_hash: Some("h".to_string()),
		role: Role::Admin,
		auth_source: AuthSource::Local,
		display_name: None,
		enabled: true,
		created_at: Utc::now(),
		updated_at: Utc::now(),
	};

	assert!(repo.create_initial_admin(&mk("1", "a")).await.unwrap());
	assert!(!repo.create_initial_admin(&mk("2", "b")).await.unwrap());
	assert_eq!(repo.count().await.unwrap(), 1);
}

#[tokio::test]
async fn create_within_quota_enforces_limit() {
	let db = memory_db().await;
	let repo = SqlxRequestRepo::new(db.clone());
	let since = Utc::now() - chrono::Duration::days(30);
	let mk = |id: &str, asin: &str| Request {
		id: id.to_string(),
		user_id: "u1".to_string(),
		asin: asin.to_string(),
		title: "t".to_string(),
		author: None,
		cover_url: None,
		status: RequestStatus::Pending,
		approved_by: None,
		notes: None,
		quality_profile_id: None,
		created_at: Utc::now(),
		updated_at: Utc::now(),
	};

	assert!(
		repo.create_within_quota(&mk("1", "x"), 2, since)
			.await
			.unwrap()
	);
	assert!(
		repo.create_within_quota(&mk("2", "y"), 2, since)
			.await
			.unwrap()
	);
	assert!(
		!repo
			.create_within_quota(&mk("3", "z"), 2, since)
			.await
			.unwrap()
	);
	assert_eq!(repo.count(Some("u1"), None).await.unwrap(), 2);
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
		notes: None,
		quality_profile_id: None,
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

	assert_eq!(requests.list_for_user("u1").await.unwrap().len(), 1);
}

#[tokio::test]
async fn request_list_page_filters_and_counts() {
	let db = memory_db().await;
	let repo = SqlxRequestRepo::new(db.clone());
	let now = Utc::now();

	let make = |id: &str, user: &str, status: RequestStatus| Request {
		id: id.to_string(),
		user_id: user.to_string(),
		asin: id.to_string(),
		title: "t".to_string(),
		author: None,
		cover_url: None,
		status,
		approved_by: None,
		notes: None,
		quality_profile_id: None,
		created_at: now,
		updated_at: now,
	};
	repo.create(&make("a", "u1", RequestStatus::Pending))
		.await
		.unwrap();
	repo.create(&make("b", "u1", RequestStatus::Approved))
		.await
		.unwrap();
	repo.create(&make("c", "u2", RequestStatus::Pending))
		.await
		.unwrap();

	assert_eq!(repo.count(None, None).await.unwrap(), 3);
	assert_eq!(repo.count(None, Some("pending")).await.unwrap(), 2);
	assert_eq!(repo.count(Some("u1"), None).await.unwrap(), 2);
	assert_eq!(repo.count(Some("u1"), Some("pending")).await.unwrap(), 1);

	assert_eq!(repo.list_page(None, None, 2, 0).await.unwrap().len(), 2);
	assert_eq!(repo.list_page(None, None, 2, 2).await.unwrap().len(), 1);

	let pending = repo.list_page(None, Some("pending"), 10, 0).await.unwrap();
	assert_eq!(pending.len(), 2);
	assert!(pending.iter().all(|r| r.status == RequestStatus::Pending));
}

#[tokio::test]
async fn status_by_asin_scopes_to_user_and_keeps_newest() {
	let db = memory_db().await;
	let repo = SqlxRequestRepo::new(db.clone());

	let make =
		|id: &str, user: &str, asin: &str, status: RequestStatus, at: chrono::DateTime<Utc>| {
			Request {
				id: id.to_string(),
				user_id: user.to_string(),
				asin: asin.to_string(),
				title: "t".to_string(),
				author: None,
				cover_url: None,
				status,
				approved_by: None,
				notes: None,
				quality_profile_id: None,
				created_at: at,
				updated_at: at,
			}
		};
	let older = Utc::now() - chrono::Duration::days(1);
	let newer = Utc::now();
	repo.create(&make("1", "u1", "asinX", RequestStatus::Declined, older))
		.await
		.unwrap();
	repo.create(&make("2", "u1", "asinX", RequestStatus::Available, newer))
		.await
		.unwrap();
	repo.create(&make("3", "u1", "asinY", RequestStatus::Pending, newer))
		.await
		.unwrap();
	repo.create(&make("4", "u2", "asinZ", RequestStatus::Pending, newer))
		.await
		.unwrap();

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let service = request_service(&db, jobs);

	let map = service.status_by_asin("u1").await.unwrap();
	assert_eq!(map.len(), 2);
	assert_eq!(map.get("asinX"), Some(&RequestStatus::Available));
	assert_eq!(map.get("asinY"), Some(&RequestStatus::Pending));
	assert!(!map.contains_key("asinZ"));

	assert_eq!(
		service.status_for_asin("u1", "asinX").await.unwrap(),
		Some(RequestStatus::Available)
	);
	assert_eq!(
		service.status_for_asin("u1", "missing").await.unwrap(),
		None
	);
}

fn caller(id: &str, role: Role) -> User {
	let now = Utc::now();
	User {
		id: id.to_string(),
		username: id.to_string(),
		email: None,
		password_hash: None,
		role,
		auth_source: AuthSource::Local,
		display_name: None,
		enabled: true,
		created_at: now,
		updated_at: now,
	}
}

fn release(download_url: &str) -> Release {
	Release {
		title: "Book m4b".to_string(),
		indexer: "trk".to_string(),
		protocol: Protocol::Torrent,
		size: 500 * 1024 * 1024,
		seeders: 10,
		leechers: 0,
		download_url: download_url.to_string(),
		info_hash: None,
		info_url: None,
		publish_date: None,
	}
}

struct FakeIndexer {
	releases: Vec<Release>,
}

#[async_trait]
impl Indexer for FakeIndexer {
	fn id(&self) -> &'static str {
		"fake"
	}
	async fn search(&self, _query: &str) -> CoreResult<Vec<Release>> {
		Ok(self.releases.clone())
	}
	async fn test_connection(&self) -> CoreResult<()> {
		Ok(())
	}
}

#[tokio::test]
async fn delete_request_cascades_to_downloads_and_activity() {
	let db = memory_db().await;
	seed_download(&db, Utc::now()).await;
	SqlxActivityRepo::new(db.clone())
		.create(&Activity {
			id: "act1".to_string(),
			request_id: "r1".to_string(),
			event: "downloading".to_string(),
			detail: None,
			actor: None,
			at: Utc::now(),
		})
		.await
		.unwrap();

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let requests = request_service(&db, jobs);
	requests
		.delete(&caller("admin", Role::Admin), "r1")
		.await
		.unwrap();

	assert!(
		SqlxRequestRepo::new(db.clone())
			.find_by_id("r1")
			.await
			.unwrap()
			.is_none()
	);
	assert!(
		SqlxDownloadRepo::new(db.clone())
			.find_by_id("d1")
			.await
			.unwrap()
			.is_none()
	);
	assert_eq!(
		SqlxActivityRepo::new(db.clone())
			.for_request("r1")
			.await
			.unwrap()
			.len(),
		0
	);
}

#[tokio::test]
async fn retry_reopens_failed_request_and_enqueues_grab() {
	let db = memory_db().await;
	let now = Utc::now();
	SqlxRequestRepo::new(db.clone())
		.create(&Request {
			id: "r1".to_string(),
			user_id: "u1".to_string(),
			asin: "B01".to_string(),
			title: "Book".to_string(),
			author: None,
			cover_url: None,
			status: RequestStatus::Failed,
			approved_by: None,
			notes: None,
			quality_profile_id: None,
			created_at: now,
			updated_at: now,
		})
		.await
		.unwrap();

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let requests = request_service(&db, jobs.clone());
	let owner = caller("u1", Role::User);

	let updated = requests.retry(&owner, "r1").await.unwrap();
	assert_eq!(updated.status, RequestStatus::Approved);

	let listed = jobs.list().await.unwrap();
	let grabs: Vec<_> = listed
		.iter()
		.filter(|job| job.job_type == JobType::Grab)
		.collect();
	assert_eq!(grabs.len(), 1);
	assert!(listed.iter().any(|job| job.job_type == JobType::Notify));

	assert!(requests.retry(&owner, "r1").await.is_err());
}

#[tokio::test]
async fn blocklisted_release_excluded_from_for_request() {
	let db = memory_db().await;
	let now = Utc::now();
	SqlxRequestRepo::new(db.clone())
		.create(&Request {
			id: "r1".to_string(),
			user_id: "u1".to_string(),
			asin: "B01".to_string(),
			title: "Book".to_string(),
			author: None,
			cover_url: None,
			status: RequestStatus::Pending,
			approved_by: None,
			notes: None,
			quality_profile_id: None,
			created_at: now,
			updated_at: now,
		})
		.await
		.unwrap();

	let indexer = Arc::new(FakeIndexer {
		releases: vec![release("magnet:a"), release("magnet:b")],
	});
	let releases = ReleaseService::new(
		indexer,
		Arc::new(SqlxQualityProfileRepo::new(db.clone())),
		Arc::new(SqlxRequestRepo::new(db.clone())),
		Arc::new(SqlxBlocklistRepo::new(db.clone())),
	);

	assert_eq!(releases.for_request("r1").await.unwrap().len(), 2);

	releases.blocklist_release("r1", "magnet:a").await.unwrap();
	let after = releases.for_request("r1").await.unwrap();
	assert_eq!(after.len(), 1);
	assert_eq!(after[0].release.download_url, "magnet:b");
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
		enabled: true,
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

#[tokio::test]
async fn download_create_and_set_state() {
	let db = memory_db().await;
	let repo = SqlxDownloadRepo::new(db.clone());

	let now = Utc::now();
	let download = Download {
		id: "d1".to_string(),
		request_id: "r1".to_string(),
		client: "qbittorrent".to_string(),
		category: "lunu".to_string(),
		release_title: "The Hobbit [M4B]".to_string(),
		indexer: "MyTracker".to_string(),
		download_url: "magnet:?xt=urn:btih:abc".to_string(),
		info_hash: Some("abc".to_string()),
		state: DownloadState::Queued,
		progress: 0,
		created_at: now,
		updated_at: now,
	};
	repo.create(&download).await.unwrap();

	let found = repo.find_by_request("r1").await.unwrap().unwrap();
	assert_eq!(found.id, "d1");
	assert_eq!(found.state, DownloadState::Queued);
	assert_eq!(found.release_title, "The Hobbit [M4B]");
	assert_eq!(found.info_hash.as_deref(), Some("abc"));

	repo.update_status("d1", DownloadState::Downloading, 42, Utc::now())
		.await
		.unwrap();
	let updated = repo.find_by_id("d1").await.unwrap().unwrap();
	assert_eq!(updated.state, DownloadState::Downloading);
	assert_eq!(updated.progress, 42);
	assert_eq!(repo.list().await.unwrap().len(), 1);
}

struct StubProvider;

#[async_trait]
impl MetadataProvider for StubProvider {
	fn id(&self) -> &'static str {
		"stub"
	}
	async fn search(&self, _query: &str, _region: &str, _page: i64) -> CoreResult<Vec<Book>> {
		Ok(Vec::new())
	}
	async fn get_book(&self, _asin: &str, _region: &str) -> CoreResult<Option<Book>> {
		Ok(None)
	}
	async fn get_chapters(&self, _asin: &str, _region: &str) -> CoreResult<Option<Chapters>> {
		Ok(None)
	}
	async fn similar(&self, _asin: &str, _region: &str) -> CoreResult<Vec<Book>> {
		Ok(Vec::new())
	}
	async fn books_by_author(&self, _author_asin: &str, _region: &str) -> CoreResult<Vec<Book>> {
		Ok(Vec::new())
	}
}

#[tokio::test]
async fn approving_a_request_enqueues_a_grab_job() {
	let db = memory_db().await;

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let requests = request_service(&db, jobs.clone());

	let now = Utc::now();
	let request = Request {
		id: "r1".to_string(),
		user_id: "u1".to_string(),
		asin: "B01".to_string(),
		title: "The Hobbit".to_string(),
		author: None,
		cover_url: None,
		status: RequestStatus::Pending,
		approved_by: None,
		notes: None,
		quality_profile_id: None,
		created_at: now,
		updated_at: now,
	};
	SqlxRequestRepo::new(db.clone())
		.create(&request)
		.await
		.unwrap();

	let approved = requests.approve("admin", "r1").await.unwrap();
	assert_eq!(approved.status, RequestStatus::Approved);

	let listed = jobs.list().await.unwrap();
	let grabs: Vec<_> = listed
		.iter()
		.filter(|job| job.job_type == JobType::Grab)
		.collect();
	assert_eq!(grabs.len(), 1);
	assert!(grabs[0].payload.contains("r1"));
	assert!(listed.iter().any(|job| job.job_type == JobType::Notify));
}

#[tokio::test]
async fn marking_available_enqueues_a_notification() {
	let db = memory_db().await;
	let now = Utc::now();
	SqlxRequestRepo::new(db.clone())
		.create(&Request {
			id: "r1".to_string(),
			user_id: "u1".to_string(),
			asin: "B01".to_string(),
			title: "The Hobbit".to_string(),
			author: None,
			cover_url: None,
			status: RequestStatus::Importing,
			approved_by: None,
			notes: None,
			quality_profile_id: None,
			created_at: now,
			updated_at: now,
		})
		.await
		.unwrap();
	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let requests = request_service(&db, jobs.clone());

	requests.mark_available("r1").await.unwrap();

	let listed = jobs.list().await.unwrap();
	let notifies: Vec<_> = listed
		.iter()
		.filter(|job| job.job_type == JobType::Notify)
		.collect();
	assert_eq!(notifies.len(), 1);
	assert!(notifies[0].payload.contains("request-available"));
	assert!(notifies[0].payload.contains("The Hobbit"));
}

#[derive(Default)]
struct RecordingNotifier {
	events: std::sync::Mutex<Vec<String>>,
}

#[async_trait]
impl Notifier for RecordingNotifier {
	fn id(&self) -> &'static str {
		"recording"
	}
	async fn deliver(&self, event: &NotificationEvent) -> CoreResult<()> {
		self.events.lock().unwrap().push(event.message());
		Ok(())
	}
}

#[tokio::test]
async fn notification_service_dispatches_to_every_notifier() {
	let a = Arc::new(RecordingNotifier::default());
	let b = Arc::new(RecordingNotifier::default());
	let service = NotificationService::new(vec![a.clone(), b.clone()]);

	let event = NotificationEvent {
		kind: NotificationKind::RequestAvailable,
		request_id: "r1".to_string(),
		title: "Dune".to_string(),
		user_id: "u1".to_string(),
	};
	service.dispatch(&event).await.unwrap();

	assert_eq!(
		a.events.lock().unwrap().as_slice(),
		&["Now available: Dune"]
	);
	assert_eq!(
		b.events.lock().unwrap().as_slice(),
		&["Now available: Dune"]
	);
}

struct FakeClient {
	response: Option<DownloadStatus>,
}

#[async_trait]
impl DownloadClient for FakeClient {
	fn id(&self) -> &'static str {
		"fake"
	}
	async fn add(&self, _download_url: &str, _category: &str) -> CoreResult<()> {
		Ok(())
	}
	async fn status(&self, _info_hash: &str) -> CoreResult<Option<DownloadStatus>> {
		Ok(self.response.clone())
	}
	async fn remove(&self, _info_hash: &str, _delete_files: bool) -> CoreResult<()> {
		Ok(())
	}
	async fn test_connection(&self) -> CoreResult<()> {
		Ok(())
	}
}

#[derive(Default)]
struct FakeImporter {
	call: std::sync::Mutex<Option<(String, String)>>,
}

#[async_trait]
impl Importer for FakeImporter {
	async fn import(&self, source: &str, destination: &str) -> CoreResult<()> {
		*self.call.lock().unwrap() = Some((source.to_string(), destination.to_string()));
		Ok(())
	}
}

fn settings_service(db: &Db) -> Arc<SettingsService> {
	let encryptor = Encryptor::new("dev-master-key-value", SETTINGS_ENCRYPTION_CONTEXT).unwrap();
	Arc::new(SettingsService::new(
		Arc::new(SqlxSettingsRepo::new(db.clone())),
		encryptor,
	))
}

#[tokio::test]
async fn import_places_content_and_marks_available() {
	let db = memory_db().await;
	seed_download(&db, Utc::now()).await;

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let settings = settings_service(&db);
	settings.set("library_dir", "/library").await.unwrap();
	let importer = Arc::new(FakeImporter::default());
	let imports = ImportService::new(
		Arc::new(SqlxDownloadRepo::new(db.clone())),
		request_service(&db, jobs),
		settings,
		importer.clone(),
		Arc::new(MediaService::new(Arc::new(SqlxMediaRepo::new(db.clone())))),
	);

	imports.import("d1", "/downloads/The Hobbit").await.unwrap();

	let call = importer.call.lock().unwrap().clone().unwrap();
	assert_eq!(call.0, "/downloads/The Hobbit");
	assert_eq!(call.1, "/library/Unknown Author/The Hobbit");
	assert_eq!(
		SqlxRequestRepo::new(db.clone())
			.find_by_id("r1")
			.await
			.unwrap()
			.unwrap()
			.status,
		RequestStatus::Available
	);
}

#[tokio::test]
async fn request_transitions_record_activity() {
	let db = memory_db().await;

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let activity = activity_service(&db);
	let requests = request_service_with_activity(&db, jobs, activity.clone());

	let now = Utc::now();
	let request = Request {
		id: "r1".to_string(),
		user_id: "u1".to_string(),
		asin: "B01".to_string(),
		title: "The Hobbit".to_string(),
		author: None,
		cover_url: None,
		status: RequestStatus::Pending,
		approved_by: None,
		notes: None,
		quality_profile_id: None,
		created_at: now,
		updated_at: now,
	};
	SqlxRequestRepo::new(db.clone())
		.create(&request)
		.await
		.unwrap();

	requests.approve("admin", "r1").await.unwrap();
	requests.mark_downloading("r1").await.unwrap();

	let events: Vec<String> = activity
		.for_request("r1")
		.await
		.unwrap()
		.into_iter()
		.map(|entry| entry.event)
		.collect();
	assert!(events.contains(&"approved".to_string()));
	assert!(events.contains(&"downloading".to_string()));
	assert_eq!(activity.list_page(10, 0).await.unwrap().len(), 2);
}

#[tokio::test]
async fn import_requires_library_configured() {
	let db = memory_db().await;
	seed_download(&db, Utc::now()).await;

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let imports = ImportService::new(
		Arc::new(SqlxDownloadRepo::new(db.clone())),
		request_service(&db, jobs),
		settings_service(&db),
		Arc::new(FakeImporter::default()),
		Arc::new(MediaService::new(Arc::new(SqlxMediaRepo::new(db.clone())))),
	);

	assert!(imports.import("d1", "/downloads/x").await.is_err());
}

fn request_service(db: &Db, jobs: Arc<JobService>) -> Arc<RequestService> {
	request_service_with_activity(db, jobs, activity_service(db))
}

struct NoopPublisher;

impl EventPublisher for NoopPublisher {
	fn publish(&self, _event: &lunu_core::models::LiveEvent) {}
}

#[derive(Default)]
struct RecordingPublisher {
	events: std::sync::Mutex<Vec<String>>,
}

impl EventPublisher for RecordingPublisher {
	fn publish(&self, event: &lunu_core::models::LiveEvent) {
		if let lunu_core::models::LiveEvent::Activity(activity) = event {
			self.events
				.lock()
				.unwrap()
				.push(format!("{}:{}", activity.request_id, activity.event));
		}
	}
}

fn activity_service(db: &Db) -> Arc<ActivityService> {
	Arc::new(ActivityService::new(
		Arc::new(SqlxActivityRepo::new(db.clone())),
		Arc::new(NoopPublisher),
	))
}

#[tokio::test]
async fn recording_activity_publishes_event() {
	let db = memory_db().await;
	let publisher = Arc::new(RecordingPublisher::default());
	let activity = ActivityService::new(
		Arc::new(SqlxActivityRepo::new(db.clone())),
		publisher.clone(),
	);

	activity
		.record("r1", "downloading", None, None)
		.await
		.unwrap();

	assert_eq!(
		publisher.events.lock().unwrap().as_slice(),
		&["r1:downloading".to_string()]
	);
}

fn request_service_with_activity(
	db: &Db,
	jobs: Arc<JobService>,
	activity: Arc<ActivityService>,
) -> Arc<RequestService> {
	let metadata = Arc::new(MetadataService::new(
		Arc::new(StubProvider),
		Arc::new(SqlxMetadataCacheRepo::new(db.clone())),
		settings_service(db),
	));
	Arc::new(RequestService::new(
		Arc::new(SqlxRequestRepo::new(db.clone())),
		Arc::new(SqlxUserSettingsRepo::new(db.clone())),
		metadata,
		jobs,
		activity,
		Arc::new(SqlxDownloadRepo::new(db.clone())),
		Arc::new(SqlxMediaRepo::new(db.clone())),
		Arc::new(NotificationInboxService::new(
			Arc::new(SqlxUserNotificationRepo::new(db.clone())),
			Arc::new(SqlxUserRepo::new(db.clone())),
			Arc::new(NoopPublisher),
		)),
	))
}

async fn seed_download(db: &Db, at: chrono::DateTime<Utc>) {
	SqlxRequestRepo::new(db.clone())
		.create(&Request {
			id: "r1".to_string(),
			user_id: "u1".to_string(),
			asin: "B01".to_string(),
			title: "The Hobbit".to_string(),
			author: None,
			cover_url: None,
			status: RequestStatus::Downloading,
			approved_by: Some("admin".to_string()),
			notes: None,
			quality_profile_id: None,
			created_at: at,
			updated_at: at,
		})
		.await
		.unwrap();
	SqlxDownloadRepo::new(db.clone())
		.create(&Download {
			id: "d1".to_string(),
			request_id: "r1".to_string(),
			client: "qbittorrent".to_string(),
			category: "lunu".to_string(),
			release_title: "The Hobbit [M4B]".to_string(),
			indexer: "MyTracker".to_string(),
			download_url: "magnet:?xt=urn:btih:abc".to_string(),
			info_hash: Some("abc".to_string()),
			state: DownloadState::Downloading,
			progress: 10,
			created_at: at,
			updated_at: at,
		})
		.await
		.unwrap();
}

#[tokio::test]
async fn monitor_marks_request_importing_on_completion() {
	let db = memory_db().await;
	seed_download(&db, Utc::now()).await;

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let downloads = Arc::new(SqlxDownloadRepo::new(db.clone()));
	let client = Arc::new(FakeClient {
		response: Some(DownloadStatus {
			state: DownloadState::Completed,
			progress: 1.0,
			content_path: Some("/library/x".to_string()),
		}),
	});
	let monitor = MonitorService::new(
		downloads.clone(),
		client,
		request_service(&db, jobs.clone()),
		jobs.clone(),
		Arc::new(NoopPublisher),
	);

	monitor
		.poll(&MonitorPayload {
			download_id: "d1".to_string(),
			misses: 0,
			stalls: 0,
		})
		.await
		.unwrap();

	assert_eq!(
		SqlxRequestRepo::new(db.clone())
			.find_by_id("r1")
			.await
			.unwrap()
			.unwrap()
			.status,
		RequestStatus::Importing
	);
	let download = downloads.find_by_id("d1").await.unwrap().unwrap();
	assert_eq!(download.state, DownloadState::Completed);
	assert_eq!(download.progress, 100);
}

#[tokio::test]
async fn monitor_reschedules_while_downloading() {
	let db = memory_db().await;
	seed_download(&db, Utc::now()).await;

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let downloads = Arc::new(SqlxDownloadRepo::new(db.clone()));
	let client = Arc::new(FakeClient {
		response: Some(DownloadStatus {
			state: DownloadState::Downloading,
			progress: 0.5,
			content_path: None,
		}),
	});
	let monitor = MonitorService::new(
		downloads.clone(),
		client,
		request_service(&db, jobs.clone()),
		jobs.clone(),
		Arc::new(NoopPublisher),
	);

	monitor
		.poll(&MonitorPayload {
			download_id: "d1".to_string(),
			misses: 0,
			stalls: 0,
		})
		.await
		.unwrap();

	let listed = jobs.list().await.unwrap();
	assert_eq!(listed.len(), 1);
	assert_eq!(listed[0].job_type, JobType::MonitorDownload);
	assert_eq!(
		downloads.find_by_id("d1").await.unwrap().unwrap().progress,
		50
	);
}

#[tokio::test]
async fn monitor_fails_after_max_misses() {
	let db = memory_db().await;
	seed_download(&db, Utc::now()).await;

	let jobs = Arc::new(JobService::new(Arc::new(SqlxJobRepo::new(db.clone()))));
	let downloads = Arc::new(SqlxDownloadRepo::new(db.clone()));
	let client = Arc::new(FakeClient { response: None });
	let monitor = MonitorService::new(
		downloads.clone(),
		client,
		request_service(&db, jobs.clone()),
		jobs.clone(),
		Arc::new(NoopPublisher),
	);

	monitor
		.poll(&MonitorPayload {
			download_id: "d1".to_string(),
			misses: MONITOR_MAX_MISSES - 1,
			stalls: 0,
		})
		.await
		.unwrap();

	assert_eq!(
		downloads.find_by_id("d1").await.unwrap().unwrap().state,
		DownloadState::Failed
	);
	assert_eq!(
		SqlxRequestRepo::new(db.clone())
			.find_by_id("r1")
			.await
			.unwrap()
			.unwrap()
			.status,
		RequestStatus::Failed
	);
	let listed = jobs.list().await.unwrap();
	assert!(
		!listed
			.iter()
			.any(|job| matches!(job.job_type, JobType::Grab | JobType::MonitorDownload))
	);
}

fn pending_job(id: &str, at: chrono::DateTime<Utc>) -> Job {
	Job {
		id: id.to_string(),
		job_type: JobType::Grab,
		request_id: None,
		payload: "{\"request\":\"r1\"}".to_string(),
		status: JobStatus::Pending,
		attempts: 0,
		max_attempts: 3,
		run_after: at,
		locked_by: None,
		locked_at: None,
		last_error: None,
		created_at: at,
		updated_at: at,
	}
}

#[tokio::test]
async fn job_claim_is_atomic_and_lifecycle_transitions() {
	let db = memory_db().await;
	let repo = SqlxJobRepo::new(db.clone());

	let now = Utc::now();
	repo.create(&pending_job("j1", now)).await.unwrap();

	let claimed = repo
		.claim_next("worker-a", Utc::now())
		.await
		.unwrap()
		.unwrap();
	assert_eq!(claimed.id, "j1");
	assert_eq!(claimed.status, JobStatus::Running);
	assert_eq!(claimed.attempts, 1);
	assert_eq!(claimed.locked_by.as_deref(), Some("worker-a"));

	assert!(
		repo.claim_next("worker-b", Utc::now())
			.await
			.unwrap()
			.is_none()
	);

	let future = Utc::now() + chrono::Duration::seconds(30);
	repo.reschedule("j1", "temporary", future, Utc::now())
		.await
		.unwrap();
	let after = repo.find_by_id("j1").await.unwrap().unwrap();
	assert_eq!(after.status, JobStatus::Pending);
	assert_eq!(after.attempts, 1);
	assert_eq!(after.last_error.as_deref(), Some("temporary"));
	assert!(after.locked_by.is_none());

	assert!(
		repo.claim_next("worker-a", Utc::now())
			.await
			.unwrap()
			.is_none()
	);

	let reclaimed = repo
		.claim_next("worker-a", future + chrono::Duration::seconds(1))
		.await
		.unwrap()
		.unwrap();
	assert_eq!(reclaimed.attempts, 2);

	repo.complete("j1", Utc::now()).await.unwrap();
	assert_eq!(
		repo.find_by_id("j1").await.unwrap().unwrap().status,
		JobStatus::Completed
	);
	assert_eq!(repo.list().await.unwrap().len(), 1);
}

#[tokio::test]
async fn reap_stale_returns_running_jobs_to_pending() {
	let db = memory_db().await;
	let repo = SqlxJobRepo::new(db.clone());

	let now = Utc::now();
	repo.create(&pending_job("j2", now)).await.unwrap();
	repo.claim_next("worker-a", now).await.unwrap().unwrap();

	assert_eq!(
		repo.reap_stale(now - chrono::Duration::seconds(300), Utc::now())
			.await
			.unwrap(),
		0
	);

	let reaped = repo
		.reap_stale(now + chrono::Duration::seconds(1), Utc::now())
		.await
		.unwrap();
	assert_eq!(reaped, 1);

	let after = repo.find_by_id("j2").await.unwrap().unwrap();
	assert_eq!(after.status, JobStatus::Pending);
	assert!(after.locked_by.is_none());
	assert_eq!(after.attempts, 1);
}

#[tokio::test]
async fn quality_profile_crud_and_default() {
	let db = memory_db().await;
	let repo = SqlxQualityProfileRepo::new(db.clone());

	let now = Utc::now();
	let profile = QualityProfile {
		id: "p1".to_string(),
		name: "Audiobook".to_string(),
		allowed_formats: vec!["m4b".to_string(), "mp3".to_string()],
		preferred_formats: vec!["m4b".to_string()],
		min_seeders: 2,
		min_size_mb: Some(10),
		max_size_mb: None,
		seeder_weight: 1,
		format_weight: 100,
		is_default: true,
		created_at: now,
		updated_at: now,
	};
	repo.create(&profile).await.unwrap();

	let loaded = repo.find_by_id("p1").await.unwrap().unwrap();
	assert_eq!(loaded.allowed_formats, vec!["m4b", "mp3"]);
	assert_eq!(loaded.min_seeders, 2);
	assert_eq!(loaded.min_size_mb, Some(10));
	assert!(loaded.is_default);

	assert_eq!(repo.find_default().await.unwrap().unwrap().id, "p1");

	let mut second = profile.clone();
	second.id = "p2".to_string();
	second.is_default = false;
	repo.create(&second).await.unwrap();

	repo.set_default("p2").await.unwrap();
	assert_eq!(repo.find_default().await.unwrap().unwrap().id, "p2");
	assert!(!repo.find_by_id("p1").await.unwrap().unwrap().is_default);
	assert_eq!(repo.list().await.unwrap().len(), 2);
}
