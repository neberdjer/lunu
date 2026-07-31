use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use lunu_core::Result;
use lunu_core::consts::reasons;
use lunu_core::consts::settings::{
	DEFAULT_OIDC_SCOPES, OIDC_CLIENT_ID, OIDC_CLIENT_SECRET, OIDC_ISSUER_URL, OIDC_SCOPES,
};
use lunu_core::services::SettingsService;
use lunu_core::traits::{OidcClaims, OidcFlow};
use serde::Deserialize;

use crate::http::{get_json, write_json};
use crate::{integration_error, optional_setting, required_setting};

const REQUEST_TIMEOUT_SECS: u64 = 15;
const DISCOVERY_PATH: &str = "/.well-known/openid-configuration";

#[derive(Deserialize)]
struct Discovery {
	issuer: String,
	authorization_endpoint: String,
	token_endpoint: String,
	userinfo_endpoint: String,
}

fn issuer_is_secure(issuer: &str) -> bool {
	if issuer.starts_with("https://") {
		return true;
	}
	let Some(rest) = issuer.strip_prefix("http://") else {
		return false;
	};
	let authority = rest.split('/').next().unwrap_or("");
	let host = match authority.strip_prefix('[') {
		Some(v6) => v6.split(']').next().unwrap_or(""),
		None => authority.split(':').next().unwrap_or(""),
	};
	matches!(host, "localhost" | "127.0.0.1" | "::1")
}

#[derive(Deserialize)]
struct TokenResponse {
	access_token: String,
}

#[derive(Deserialize)]
struct UserInfo {
	sub: String,
	preferred_username: Option<String>,
	email: Option<String>,
	email_verified: Option<bool>,
	name: Option<String>,
}

pub struct OidcClient {
	http: reqwest::Client,
	settings: Arc<SettingsService>,
}

impl OidcClient {
	pub fn new(settings: Arc<SettingsService>) -> Self {
		let http = crate::http_client_builder(Duration::from_secs(REQUEST_TIMEOUT_SECS))
			.build()
			.expect("reqwest client builds with static configuration");

		Self { http, settings }
	}

	async fn required(&self, key: &str) -> Result<String> {
		required_setting(&self.settings, key, reasons::OIDC_NOT_CONFIGURED).await
	}

	async fn discover(&self) -> Result<Discovery> {
		let issuer = self.required(OIDC_ISSUER_URL).await?;
		if !issuer_is_secure(&issuer) {
			return Err(integration_error(
				"oidc issuer must use https (http is allowed only for loopback)",
			));
		}
		let issuer = issuer.trim_end_matches('/');
		let url = format!("{issuer}{DISCOVERY_PATH}");
		let discovery: Discovery = get_json(|| self.http.get(&url)).await?;
		if discovery.issuer.trim_end_matches('/') != issuer {
			return Err(integration_error(
				"oidc discovery issuer does not match the configured issuer url",
			));
		}
		Ok(discovery)
	}

	async fn scopes(&self) -> Result<String> {
		Ok(optional_setting(&self.settings, OIDC_SCOPES)
			.await?
			.unwrap_or_else(|| DEFAULT_OIDC_SCOPES.to_string()))
	}
}

#[async_trait]
impl OidcFlow for OidcClient {
	async fn authorize_url(
		&self,
		state: &str,
		code_challenge: &str,
		redirect_uri: &str,
	) -> Result<String> {
		let discovery = self.discover().await?;
		let client_id = self.required(OIDC_CLIENT_ID).await?;
		let scopes = self.scopes().await?;

		let mut url =
			reqwest::Url::parse(&discovery.authorization_endpoint).map_err(integration_error)?;
		url.query_pairs_mut()
			.append_pair("response_type", "code")
			.append_pair("client_id", &client_id)
			.append_pair("redirect_uri", redirect_uri)
			.append_pair("scope", &scopes)
			.append_pair("state", state)
			.append_pair("code_challenge", code_challenge)
			.append_pair("code_challenge_method", "S256");
		Ok(url.to_string())
	}

	async fn exchange(&self, code: &str, verifier: &str, redirect_uri: &str) -> Result<OidcClaims> {
		let discovery = self.discover().await?;
		let client_id = self.required(OIDC_CLIENT_ID).await?;
		let client_secret = optional_setting(&self.settings, OIDC_CLIENT_SECRET).await?;

		let form = [
			("grant_type", "authorization_code"),
			("code", code),
			("redirect_uri", redirect_uri),
			("client_id", &client_id),
			("code_verifier", verifier),
		];
		let token: TokenResponse = write_json(|| {
			let request = self.http.post(&discovery.token_endpoint).form(&form);
			match &client_secret {
				Some(secret) => request.basic_auth(&client_id, Some(secret)),
				None => request,
			}
		})
		.await?;

		let info: UserInfo = get_json(|| {
			self.http
				.get(&discovery.userinfo_endpoint)
				.bearer_auth(&token.access_token)
		})
		.await?;

		Ok(OidcClaims {
			subject: info.sub,
			username: info.preferred_username,
			email: info.email,
			email_verified: info.email_verified.unwrap_or(false),
			display_name: info.name,
		})
	}
}
