use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::consts::reasons;
use crate::crypto::{generate_token, hash_token};
use crate::models::{Invite, Role};
use crate::repo::InviteRepo;
use crate::services::{SettingsService, new_id, nonempty};
use crate::traits::Mailer;
use crate::{Error, Result};

pub struct IssuedInvite {
	pub invite: Invite,
	pub code: String,
}

pub struct InviteService {
	invites: Arc<dyn InviteRepo>,
	mailer: Arc<dyn Mailer>,
	settings: Arc<SettingsService>,
}

impl InviteService {
	pub fn new(
		invites: Arc<dyn InviteRepo>,
		mailer: Arc<dyn Mailer>,
		settings: Arc<SettingsService>,
	) -> Self {
		Self {
			invites,
			mailer,
			settings,
		}
	}

	pub async fn create(
		&self,
		created_by: &str,
		role: Role,
		email: Option<String>,
		max_uses: i64,
		expires_at: Option<DateTime<Utc>>,
	) -> Result<IssuedInvite> {
		if max_uses < 1 {
			return Err(Error::Validation(reasons::INVITE_MAX_USES.to_string()));
		}

		let code = generate_token();
		let invite = Invite {
			id: new_id(),
			code_hash: hash_token(&code),
			role,
			email,
			created_by: created_by.to_string(),
			max_uses,
			used_count: 0,
			created_at: Utc::now(),
			expires_at,
		};

		self.invites.create(&invite).await?;
		self.deliver(&invite, &code).await;
		Ok(IssuedInvite { invite, code })
	}

	async fn deliver(&self, invite: &Invite, code: &str) {
		let Some(to) = nonempty(invite.email.clone()) else {
			return;
		};
		let link = self
			.settings
			.app_link(&format!("invite/{code}"))
			.await
			.ok()
			.flatten();
		let locale = lunu_i18n::default_locale();
		let rendered = crate::email::invite(&locale, code, link.as_deref());
		let _ = self.mailer.send(&to, &rendered).await;
	}

	pub async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<Invite>> {
		self.invites.list_page(limit, offset).await
	}

	pub async fn count(&self) -> Result<i64> {
		self.invites.count().await
	}

	pub async fn delete(&self, id: &str) -> Result<()> {
		self.invites.delete(id).await
	}
}
