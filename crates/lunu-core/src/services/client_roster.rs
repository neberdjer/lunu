use std::sync::Arc;

use crate::consts::reasons;
use crate::models::Protocol;
use crate::traits::DownloadClient;
use crate::{Error, Result};

#[derive(Clone)]
pub struct ClientRoster(Vec<Arc<dyn DownloadClient>>);

impl ClientRoster {
	pub fn new(clients: Vec<Arc<dyn DownloadClient>>) -> Self {
		Self(clients)
	}

	pub fn by_id(&self, client_id: &str) -> Result<&Arc<dyn DownloadClient>> {
		self.0
			.iter()
			.find(|client| client.id() == client_id)
			.ok_or_else(|| Error::NotFound(format!("download client {client_id}")))
	}

	pub fn by_protocol(&self, protocol: Protocol) -> Result<&Arc<dyn DownloadClient>> {
		self.0
			.iter()
			.find(|client| client.protocol() == protocol)
			.ok_or_else(|| Error::Validation(reasons::NO_CLIENT_FOR_PROTOCOL.to_string()))
	}
}
