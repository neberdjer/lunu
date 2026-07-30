use super::builders::*;
use super::*;

fn invite_service_with_mailer(db: &Db, mailer: Arc<RecordingMailer>) -> InviteService {
	InviteService::new(
		Arc::new(SqlxInviteRepo::new(db.clone())),
		mailer,
		settings_service(db),
	)
}

#[tokio::test]
async fn an_invite_with_an_email_is_delivered() {
	let db = memory_db().await;
	let auth = auth_service(&db);
	let admin = auth
		.setup_first_admin("admin", "password123", None)
		.await
		.unwrap();
	let mailer = Arc::new(RecordingMailer::default());
	let invites = invite_service_with_mailer(&db, mailer.clone());

	invites
		.create(
			&admin.user.id,
			Role::User,
			Some("bob@example.com".to_string()),
			1,
			None,
		)
		.await
		.unwrap();

	assert_eq!(
		mailer.count(),
		1,
		"an invite carrying an address must mail the invitee the code"
	);
}

#[tokio::test]
async fn an_invite_without_an_email_sends_nothing() {
	let db = memory_db().await;
	let auth = auth_service(&db);
	let admin = auth
		.setup_first_admin("admin", "password123", None)
		.await
		.unwrap();
	let mailer = Arc::new(RecordingMailer::default());
	let invites = invite_service_with_mailer(&db, mailer.clone());

	invites
		.create(&admin.user.id, Role::User, None, 1, None)
		.await
		.unwrap();

	assert_eq!(
		mailer.count(),
		0,
		"a link-only invite has no recipient, so nothing is mailed"
	);
}
