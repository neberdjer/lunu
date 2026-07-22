use std::sync::Arc;

use lunu_core::services::{ClientRoster, SettingsService};
use lunu_core::traits::MetadataProvider;
use lunu_integrations::download::{QbittorrentClient, SabnzbdClient, TransmissionClient};
use lunu_integrations::metadata::{
	AudnexusProvider, GoogleBooksProvider, HardcoverProvider, OpenLibraryProvider,
};

pub(crate) fn metadata_providers(
	settings: &Arc<SettingsService>,
) -> Vec<Arc<dyn MetadataProvider>> {
	vec![
		Arc::new(AudnexusProvider::new(settings.clone())),
		Arc::new(OpenLibraryProvider::new()),
		Arc::new(GoogleBooksProvider::new(settings.clone())),
		Arc::new(HardcoverProvider::new(settings.clone())),
	]
}

pub(crate) fn download_clients(settings: &Arc<SettingsService>) -> ClientRoster {
	ClientRoster::new(vec![
		Arc::new(QbittorrentClient::new(settings.clone())),
		Arc::new(TransmissionClient::new(settings.clone())),
		Arc::new(SabnzbdClient::new(settings.clone())),
	])
}
