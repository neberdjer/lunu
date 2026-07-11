use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::consts::reasons;
use crate::crypto::{generate_token, hash_token};
use crate::models::{Invite, Role};
use crate::repo::InviteRepo;
use crate::services::new_id;
use crate::{Error, Result};

pub struct IssuedInvite {
	pub invite: Invite,
	pub code: String,
}

pub struct InviteService {
	invites: Arc<dyn InviteRepo>,
}

impl InviteService {
	pub fn new(invites: Arc<dyn InviteRepo>) -> Self {
		Self { invites }
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
		Ok(IssuedInvite { invite, code })
	}

	pub async fn list(&self) -> Result<Vec<Invite>> {
		self.invites.list().await
	}

	pub async fn delete(&self, id: &str) -> Result<()> {
		self.invites.delete(id).await
	}
}
