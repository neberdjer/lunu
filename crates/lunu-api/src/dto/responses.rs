use serde::Serialize;

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct StatusResponse {
	pub status: String,
}

impl StatusResponse {
	pub(crate) fn new(status: impl Into<String>) -> Self {
		Self {
			status: status.into(),
		}
	}
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct JobQueuedResponse {
	pub status: String,
	pub job_id: String,
}

impl JobQueuedResponse {
	pub(crate) fn queued(job_id: String) -> Self {
		Self {
			status: "queued".to_string(),
			job_id,
		}
	}
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct MergeAllResponse {
	pub status: String,
	pub queued: usize,
	pub truncated: bool,
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct SetupStatusResponse {
	pub needs_setup: bool,
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct IntegrationOkResponse {
	pub ok: bool,
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct SettingViewResponse {
	pub key: String,
	pub secret: bool,
	pub value: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct SettingSpecResponse {
	pub key: String,
	pub kind: String,
	pub choices: Vec<String>,
	pub secret: bool,
	pub default: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct SettingsCatalogResponse {
	pub keys: Vec<String>,
	pub catalog: Vec<SettingSpecResponse>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct EnqueuedResponse {
	pub enqueued: usize,
}
