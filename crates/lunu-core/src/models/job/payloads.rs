use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrabPayload {
	pub request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorPayload {
	pub download_id: String,
	pub misses: i64,
	#[serde(default)]
	pub stalls: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPayload {
	pub download_id: String,
	pub content_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergePayload {
	pub media_id: String,
}
