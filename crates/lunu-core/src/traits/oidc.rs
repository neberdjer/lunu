use async_trait::async_trait;

use crate::Result;

#[derive(Debug, Clone)]
pub struct OidcClaims {
	pub subject: String,
	pub username: Option<String>,
	pub email: Option<String>,
	pub email_verified: bool,
	pub display_name: Option<String>,
}

impl OidcClaims {
	pub fn preferred_name(&self) -> String {
		if let Some(username) = self.username.as_deref().filter(|name| !name.is_empty()) {
			return username.to_string();
		}
		if let Some(local) = self
			.email
			.as_deref()
			.and_then(|email| email.split('@').next())
			.filter(|local| !local.is_empty())
		{
			return local.to_string();
		}
		format!("user-{}", self.subject.chars().take(8).collect::<String>())
	}
}

#[async_trait]
pub trait OidcFlow: Send + Sync {
	async fn authorize_url(
		&self,
		state: &str,
		code_challenge: &str,
		redirect_uri: &str,
	) -> Result<String>;
	async fn exchange(&self, code: &str, verifier: &str, redirect_uri: &str) -> Result<OidcClaims>;
}
