use lunu_core::models::AuthSource;
use lunu_core::traits::{OidcClaims, OidcFlow};

use super::builders::*;
use super::*;

struct StubFlow {
	claims: OidcClaims,
}

impl StubFlow {
	fn for_subject(subject: &str, email: Option<&str>) -> Arc<Self> {
		Arc::new(Self {
			claims: OidcClaims {
				subject: subject.to_string(),
				username: Some("alice".to_string()),
				email: email.map(str::to_string),
				display_name: Some("Alice".to_string()),
			},
		})
	}
}

#[async_trait]
impl OidcFlow for StubFlow {
	async fn authorize_url(
		&self,
		state: &str,
		code_challenge: &str,
		redirect_uri: &str,
	) -> CoreResult<String> {
		Ok(format!(
			"https://idp/authorize?state={state}&code_challenge={code_challenge}&redirect_uri={redirect_uri}"
		))
	}

	async fn exchange(
		&self,
		_code: &str,
		_verifier: &str,
		_redirect_uri: &str,
	) -> CoreResult<OidcClaims> {
		Ok(self.claims.clone())
	}
}

async fn oidc_auth(db: &Db, flow: Arc<StubFlow>) -> AuthService {
	let auth = auth_service(db).with_oidc(flow);
	settings_service(db)
		.set("oidc_issuer_url", "https://idp")
		.await
		.unwrap();
	settings_service(db)
		.set("oidc_client_id", "lunu")
		.await
		.unwrap();
	settings_service(db)
		.set("base_url", "https://lunu.example")
		.await
		.unwrap();
	auth
}

async fn seed_local_user(db: &Db, username: &str, email: &str) {
	let now = Utc::now();
	SqlxUserRepo::new(db.clone())
		.create(&lunu_core::models::User {
			id: format!("user-{username}"),
			username: username.to_string(),
			email: Some(email.to_string()),
			display_name: None,
			locale: None,
			password_hash: Some("hash".to_string()),
			role: lunu_core::models::Role::User,
			auth_source: AuthSource::Local,
			oidc_subject: None,
			enabled: true,
			email_verified: true,
			created_at: now,
			updated_at: now,
		})
		.await
		.unwrap();
}

fn state_from(url: &str) -> String {
	url.split("state=")
		.nth(1)
		.unwrap()
		.split('&')
		.next()
		.unwrap()
		.to_string()
}

async fn started(auth: &AuthService) -> (String, String) {
	let start = auth.oidc_start().await.unwrap();
	(state_from(&start.url), start.binding)
}

#[tokio::test]
async fn the_start_url_carries_state_challenge_and_callback() {
	let db = memory_db().await;
	let auth = oidc_auth(&db, StubFlow::for_subject("sub-1", None)).await;

	let start = auth.oidc_start().await.unwrap();
	assert!(start.url.contains("state="));
	assert!(start.url.contains("code_challenge="));
	assert!(
		start
			.url
			.contains("redirect_uri=https://lunu.example/api/v1/auth/oidc/callback")
	);
	assert!(!start.binding.is_empty());
}

#[tokio::test]
async fn a_callback_provisions_once_and_reuses_by_subject() {
	let db = memory_db().await;
	let auth = oidc_auth(
		&db,
		StubFlow::for_subject("sub-1", Some("alice@example.com")),
	)
	.await;

	let (state, binding) = started(&auth).await;
	let first = auth.oidc_callback(&state, "code", &binding).await.unwrap();
	assert_eq!(first.user.username, "alice");
	assert_eq!(first.user.auth_source, AuthSource::Oidc);
	assert_eq!(first.user.oidc_subject.as_deref(), Some("sub-1"));

	let (state, binding) = started(&auth).await;
	let again = auth.oidc_callback(&state, "code", &binding).await.unwrap();
	assert_eq!(
		again.user.id, first.user.id,
		"the subject is the identity, not the username"
	);
}

#[tokio::test]
async fn a_state_is_single_use_and_unknown_states_are_rejected() {
	let db = memory_db().await;
	let auth = oidc_auth(&db, StubFlow::for_subject("sub-1", None)).await;

	let (state, binding) = started(&auth).await;
	auth.oidc_callback(&state, "code", &binding).await.unwrap();

	let Err(replay) = auth.oidc_callback(&state, "code", &binding).await else {
		panic!("a state must be single use");
	};
	assert!(matches!(replay, Error::Validation(reason) if reason == "oidc-state-invalid"));

	let Err(unknown) = auth.oidc_callback("forged", "code", &binding).await else {
		panic!("an unknown state must be rejected");
	};
	assert!(matches!(unknown, Error::Validation(reason) if reason == "oidc-state-invalid"));
}

#[tokio::test]
async fn an_existing_email_refuses_silent_linking() {
	let db = memory_db().await;
	let auth = oidc_auth(
		&db,
		StubFlow::for_subject("sub-1", Some("admin@example.com")),
	)
	.await;
	seed_local_user(&db, "admin", "admin@example.com").await;

	let (state, binding) = started(&auth).await;
	let Err(error) = auth.oidc_callback(&state, "code", &binding).await else {
		panic!("an idp asserting a known email must not take over the local account");
	};
	assert!(matches!(error, Error::Conflict(reason) if reason == "oidc-account-conflict"));
}

#[tokio::test]
async fn a_taken_username_gets_a_suffix_instead_of_a_collision() {
	let db = memory_db().await;
	let auth = oidc_auth(&db, StubFlow::for_subject("sub-2", None)).await;
	seed_local_user(&db, "alice", "other@example.com").await;

	let (state, binding) = started(&auth).await;
	let authenticated = auth.oidc_callback(&state, "code", &binding).await.unwrap();
	assert_ne!(authenticated.user.username, "alice");
	assert!(authenticated.user.username.starts_with("alice-"));
}

#[tokio::test]
async fn start_requires_configuration() {
	let db = memory_db().await;
	let auth = auth_service(&db).with_oidc(StubFlow::for_subject("sub-1", None));

	let error = auth.oidc_start().await.unwrap_err();
	assert!(matches!(error, Error::Validation(reason) if reason == "oidc-not-configured"));
}

#[tokio::test]
async fn a_state_from_another_browser_is_rejected() {
	let db = memory_db().await;
	let auth = oidc_auth(&db, StubFlow::for_subject("sub-1", None)).await;

	let (state, _) = started(&auth).await;
	let Err(error) = auth
		.oidc_callback(&state, "code", "someone-elses-binding")
		.await
	else {
		panic!("a callback without the initiating browser's binding must fail");
	};
	assert!(matches!(error, Error::Validation(reason) if reason == "oidc-state-invalid"));

	let Err(replayed) = auth.oidc_callback(&state, "code", "still-wrong").await else {
		panic!("a binding miss must also consume the state");
	};
	assert!(matches!(replayed, Error::Validation(reason) if reason == "oidc-state-invalid"));
}

#[tokio::test]
async fn a_proxy_user_is_provisioned_once() {
	let db = memory_db().await;
	let auth = auth_service(&db);

	let first = auth.proxy_user("bob").await.unwrap();
	assert_eq!(first.auth_source, AuthSource::Proxy);

	let again = auth.proxy_user("bob").await.unwrap();
	assert_eq!(again.id, first.id);
}
