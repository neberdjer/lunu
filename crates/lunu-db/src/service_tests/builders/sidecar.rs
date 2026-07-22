use super::*;

#[derive(Default)]
pub(crate) struct RecordingSidecar {
	pub(crate) written: std::sync::Mutex<Vec<(String, String, Option<String>)>>,
}

#[async_trait]
impl lunu_core::traits::SidecarWriter for RecordingSidecar {
	async fn write(&self, sidecar: &lunu_core::traits::Sidecar<'_>) -> CoreResult<()> {
		self.written.lock().unwrap().push((
			sidecar.directory.to_string(),
			sidecar.opf.to_string(),
			sidecar.cover_url.map(str::to_string),
		));
		Ok(())
	}
}
