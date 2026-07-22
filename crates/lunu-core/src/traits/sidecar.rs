use async_trait::async_trait;

use crate::Result;

pub struct Sidecar<'a> {
	pub directory: &'a str,
	pub opf: &'a str,
	pub cover_url: Option<&'a str>,
}

#[async_trait]
pub trait SidecarWriter: Send + Sync {
	async fn write(&self, sidecar: &Sidecar<'_>) -> Result<()>;
}
