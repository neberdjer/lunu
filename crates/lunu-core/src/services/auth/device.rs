use super::AuthService;
use crate::Result;
use crate::email;
use crate::models::User;
use crate::services::nonempty;

impl AuthService {
	pub async fn record_user_agent(
		&self,
		session_id: &str,
		user_agent: Option<&str>,
	) -> Result<()> {
		if let Some(user_agent) = user_agent {
			self.sessions.set_user_agent(session_id, user_agent).await?;
		}
		Ok(())
	}

	pub async fn record_login_device(
		&self,
		user: &User,
		session_id: &str,
		user_agent: Option<&str>,
		accept_language: Option<&str>,
	) -> Result<()> {
		let Some(user_agent) = user_agent else {
			return Ok(());
		};
		self.sessions.set_user_agent(session_id, user_agent).await?;

		let sessions = self.sessions.list_for_user(&user.id).await?;
		let others: Vec<_> = sessions
			.iter()
			.filter(|session| session.id != session_id)
			.collect();
		if others.is_empty()
			|| others
				.iter()
				.any(|session| session.user_agent.as_deref() == Some(user_agent))
		{
			return Ok(());
		}

		let Some(recipient) = nonempty(user.email.clone()) else {
			return Ok(());
		};

		let locale = lunu_i18n::negotiate(accept_language, user.locale.as_deref());
		let rendered = email::new_device(&locale, user_agent);
		let _ = self
			.mailer
			.send(&recipient, &rendered.subject, &rendered.html)
			.await;
		Ok(())
	}
}
