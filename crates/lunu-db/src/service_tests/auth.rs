use super::builders::*;
use super::*;

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

	let first = expect_session(auth.login("absuser", "abspass").await.unwrap());
	assert_eq!(first.user.auth_source, AuthSource::Abs);
	assert_eq!(first.user.email.as_deref(), Some("abs@example.com"));
	assert!(first.user.password_hash.is_none());
	assert!(
		auth.validate_session(&first.session_token)
			.await
			.unwrap()
			.is_some()
	);

	let second = expect_session(auth.login("absuser", "abspass").await.unwrap());
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
async fn abs_credentials_cannot_log_in_an_oidc_account() {
	let db = memory_db().await;
	let mut oidc_user = caller("alice", Role::User);
	oidc_user.auth_source = AuthSource::Oidc;
	oidc_user.oidc_subject = Some("sub-alice".to_string());
	SqlxUserRepo::new(db.clone())
		.create(&oidc_user)
		.await
		.unwrap();

	let provider = Arc::new(FakeAuthProvider {
		username: "alice".to_string(),
		password: "abspass".to_string(),
		identity: ExternalIdentity {
			username: "alice".to_string(),
			email: None,
		},
	});
	let auth = auth_service_with_provider(&db, provider);

	assert!(
		matches!(
			auth.login("alice", "abspass").await,
			Err(Error::Unauthorized)
		),
		"an ABS password must never authenticate an account owned by another provider"
	);
}

#[tokio::test]
async fn forward_auth_cannot_adopt_a_non_proxy_account() {
	let db = memory_db().await;
	SqlxUserRepo::new(db.clone())
		.create(&caller("admin", Role::Admin))
		.await
		.unwrap();
	let auth = auth_service(&db);

	assert!(
		matches!(auth.proxy_user("admin").await, Err(Error::Unauthorized)),
		"a proxy-asserted username must not adopt an existing local/admin account"
	);

	let provisioned = auth.proxy_user("bob").await.unwrap();
	assert_eq!(provisioned.auth_source, AuthSource::Proxy);
	let again = auth.proxy_user("bob").await.unwrap();
	assert_eq!(again.id, provisioned.id, "an existing proxy user is reused");
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
	let authed = expect_session(auth.login("admin", "password123").await.unwrap());
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
	let invites = invite_service(&db);

	let admin = auth
		.setup_first_admin("admin", "password123", None)
		.await
		.unwrap();
	let issued = invites
		.create(&admin.user.id, Role::User, None, 1, None)
		.await
		.unwrap();

	let registered = auth
		.register_with_invite(&issued.code, "bob", "hunter2password", None)
		.await
		.unwrap();
	let lunu_core::services::Registration::Active(registered) = registered else {
		panic!("expected active registration");
	};
	assert_eq!(registered.user.username, "bob");
	assert_eq!(registered.user.role, Role::User);

	assert!(
		auth.register_with_invite(&issued.code, "carol", "password123", None)
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
async fn weak_password_does_not_burn_single_use_invite() {
	let db = memory_db().await;
	let auth = auth_service(&db);
	let invites = invite_service(&db);

	let admin = auth
		.setup_first_admin("admin", "password123", None)
		.await
		.unwrap();
	let issued = invites
		.create(&admin.user.id, Role::User, None, 1, None)
		.await
		.unwrap();

	assert!(matches!(
		auth.register_with_invite(&issued.code, "bob", "short", None)
			.await,
		Err(Error::Validation(_))
	));

	let retry = auth
		.register_with_invite(&issued.code, "bob", "hunter2password", None)
		.await
		.unwrap();
	assert!(matches!(
		retry,
		lunu_core::services::Registration::Active(_)
	));
}
