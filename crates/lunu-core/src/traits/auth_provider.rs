use async_trait::async_trait;

use crate::Result;

#[derive(Debug, Clone)]
pub struct ExternalIdentity {
	pub username: String,
	pub email: Option<String>,
}

#[async_trait]
pub trait AuthProvider: Send + Sync {
	fn name(&self) -> &'static str;
	async fn authenticate(
		&self,
		username: &str,
		password: &str,
	) -> Result<Option<ExternalIdentity>>;
}
