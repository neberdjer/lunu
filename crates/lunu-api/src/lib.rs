mod client_ip;
mod cookie;
mod dto;
mod error;
mod expiry;
mod extract;
mod hub;
mod locale;
mod log_control;
mod middleware;
mod openapi;
mod pagination;
mod rate_limit;
mod rosters;
mod state;
mod v1;
mod wire;

pub const MAX_JSON_BODY_BYTES: usize = 256 * 1024;

pub use log_control::LogControl;
pub use lunu_core::consts::api::API_PREFIX;
pub use middleware::{normalize_errors, security_headers};
pub use openapi::ApiDoc;
pub use state::AppState;
pub use v1::configure;
