use lunu_core::consts::auth::{API_KEY_HEADER, SESSION_COOKIE};
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::{Modify, OpenApi};

#[derive(OpenApi)]
#[openapi(
	info(
		title = "Lunu API",
		description = "Self-hosted audiobook request and fulfillment API."
	),
	security(("session" = []), ("api_key" = [])),
	modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
	fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
		let components = openapi
			.components
			.as_mut()
			.expect("components are initialized");
		components.add_security_scheme(
			"session",
			SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new(SESSION_COOKIE))),
		);
		components.add_security_scheme(
			"api_key",
			SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new(API_KEY_HEADER))),
		);
	}
}
