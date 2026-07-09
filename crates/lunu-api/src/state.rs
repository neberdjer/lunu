use std::sync::Arc;

use lunu_config::BootstrapConfig;
use lunu_db::Db;

#[derive(Clone)]
pub struct AppState {
	pub db: Db,
	pub config: Arc<BootstrapConfig>,
	pub version: &'static str,
}

impl AppState {
	pub fn new(db: Db, config: BootstrapConfig, version: &'static str) -> Self {
		Self {
			db,
			config: Arc::new(config),
			version,
		}
	}
}
