use crate::repos::{
	SqlxActivityRepo, SqlxApiKeyRepo, SqlxBlocklistRepo, SqlxDownloadRepo,
	SqlxEmailVerificationRepo, SqlxInviteRepo, SqlxJobRepo, SqlxMediaRepo, SqlxMetadataCacheRepo,
	SqlxMfaRecoveryCodeRepo, SqlxNotificationDeliveryRepo, SqlxPasswordResetRepo,
	SqlxQualityProfileRepo, SqlxRequestRepo, SqlxScheduleRepo, SqlxSessionRepo, SqlxSettingsRepo,
	SqlxUserMfaRepo, SqlxUserNotificationRepo, SqlxUserRepo, SqlxUserSettingsRepo, SqlxWatchRepo,
	SqlxWorkRepo,
};
use crate::{Db, run_migrations};
use async_trait::async_trait;
use chrono::Utc;
use lunu_core::consts::crypto::{MFA_ENCRYPTION_CONTEXT, SETTINGS_ENCRYPTION_CONTEXT};
use lunu_core::consts::download::MONITOR_MAX_MISSES;
use lunu_core::crypto::{Encryptor, hash_token};
use lunu_core::models::{
	Activity, ActivityTarget, AuthSource, Book, Chapters, Download, DownloadState, DownloadStatus,
	EmailVerificationToken, ExternalId, Format, IdScheme, Job, JobStatus, JobType, Media,
	MediaSource, MergeState, MetadataCacheEntry, MonitorPayload, NotificationEvent,
	NotificationKind, PasswordResetToken, Protocol, QualityProfile, Release, Request,
	RequestStatus, Role, SeriesSummary, User, UserSettings,
};
use lunu_core::repo::{
	ActivityRepo, DownloadRepo, EmailVerificationRepo, JobRepo, MetadataCacheRepo,
	NotificationDeliveryRepo, PasswordResetRepo, QualityProfileRepo, RequestRepo, SettingsRepo,
	UserRepo, UserSettingsRepo,
};
use lunu_core::services::{
	ActivityService, ApiKeyService, AuthService, ImportService, InviteService, JobService,
	MediaService, MetadataService, MonitorService, NotificationInboxService, NotificationService,
	ReleaseService, RequestService, SettingsService, UserService, WorkService,
};
use lunu_core::traits::{
	AuthProvider, DownloadClient, EventPublisher, ExternalIdentity, Importer, Indexer, Mailer,
	MetadataProvider, Notifier,
};
use lunu_core::{Error, Result as CoreResult};
use sqlx::any::{AnyPoolOptions, install_default_drivers};
use std::sync::Arc;

mod account;
mod admin;
mod approve;
mod auth;
mod builders;
mod concurrency;
mod data;
mod grab;
mod import;
mod invite;
mod library;
mod merge;
mod metadata;
mod mfa;
mod monitor;
mod monitor_complete;
mod monitor_removal;
mod notify;
mod oidc;
mod pipeline;
mod repos;
mod repos_jobs;
mod requests;
mod reset;
mod scheduler;
mod stubs;
mod verification;
mod watch;
mod works;

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

async fn concurrent_db(connections: u32) -> Option<Db> {
	install_default_drivers();
	let url = std::env::var("LUNU_TEST_DATABASE_URL").ok()?;
	Some(postgres_isolated_db_with(&url, connections).await)
}

async fn postgres_isolated_db(url: &str) -> Db {
	postgres_isolated_db_with(url, 1).await
}

async fn postgres_isolated_db_with(url: &str, connections: u32) -> Db {
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
		.max_connections(connections)
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
		"schedules",
	];

	for table in tables {
		let query = format!("SELECT COUNT(*) FROM {table}");
		sqlx::query(&query)
			.fetch_one(&db)
			.await
			.unwrap_or_else(|error| panic!("table {table} is not queryable: {error}"));
	}
}

#[tokio::test]
async fn sqlite_connections_enable_wal_so_writers_do_not_block_readers() {
	let path = std::env::temp_dir().join(format!("lunu_wal_{}.db", std::process::id()));
	let _ = std::fs::remove_file(&path);
	let url = format!("sqlite://{}?mode=rwc", path.display());

	let db = crate::connect(&url).await.unwrap();
	let mode: String = sqlx::query_scalar("PRAGMA journal_mode")
		.fetch_one(&db)
		.await
		.unwrap();
	let synchronous: i64 = sqlx::query_scalar("PRAGMA synchronous")
		.fetch_one(&db)
		.await
		.unwrap();
	db.close().await;

	let _ = std::fs::remove_file(&path);
	let _ = std::fs::remove_file(path.with_extension("db-wal"));
	let _ = std::fs::remove_file(path.with_extension("db-shm"));

	assert_eq!(
		mode.to_ascii_lowercase(),
		"wal",
		"sqlite must run in WAL mode"
	);
	assert_eq!(synchronous, 1, "WAL pairs with synchronous=NORMAL (1)");
}
