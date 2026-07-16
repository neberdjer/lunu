use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::http::header::{
	CONTENT_SECURITY_POLICY, HeaderValue, REFERRER_POLICY, STRICT_TRANSPORT_SECURITY,
	X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS,
};
use actix_web::middleware::Next;
use lunu_core::consts::security;

pub async fn security_headers<B>(
	req: ServiceRequest,
	next: Next<B>,
	hsts: bool,
) -> Result<ServiceResponse<B>, actix_web::Error>
where
	B: MessageBody,
{
	let mut response = next.call(req).await?;
	let headers = response.headers_mut();

	headers.insert(
		CONTENT_SECURITY_POLICY,
		HeaderValue::from_static(security::CONTENT_SECURITY_POLICY),
	);
	headers.insert(
		X_FRAME_OPTIONS,
		HeaderValue::from_static(security::FRAME_OPTIONS),
	);
	headers.insert(
		X_CONTENT_TYPE_OPTIONS,
		HeaderValue::from_static(security::CONTENT_TYPE_OPTIONS),
	);
	headers.insert(
		REFERRER_POLICY,
		HeaderValue::from_static(security::REFERRER_POLICY),
	);

	if hsts {
		headers.insert(
			STRICT_TRANSPORT_SECURITY,
			HeaderValue::from_static(security::STRICT_TRANSPORT_SECURITY),
		);
	}

	Ok(response)
}

#[cfg(test)]
mod tests {
	use actix_web::middleware::from_fn;
	use actix_web::test as actix_test;
	use actix_web::{App, HttpResponse, web};

	use super::*;

	async fn response_headers(hsts: bool, path: &str) -> actix_web::http::header::HeaderMap {
		let app = actix_test::init_service(
			App::new()
				.wrap(from_fn(move |req, next| security_headers(req, next, hsts)))
				.route(
					"/ok",
					web::get().to(|| async { HttpResponse::Ok().finish() }),
				)
				.route(
					"/boom",
					web::get().to(|| async { HttpResponse::InternalServerError().finish() }),
				),
		)
		.await;
		let request = actix_test::TestRequest::get().uri(path).to_request();
		actix_test::call_service(&app, request)
			.await
			.headers()
			.clone()
	}

	#[actix_web::test]
	async fn every_response_carries_the_baseline_headers() {
		let headers = response_headers(false, "/ok").await;
		assert_eq!(
			headers.get(CONTENT_SECURITY_POLICY).unwrap(),
			security::CONTENT_SECURITY_POLICY
		);
		assert_eq!(headers.get(X_FRAME_OPTIONS).unwrap(), "DENY");
		assert_eq!(headers.get(X_CONTENT_TYPE_OPTIONS).unwrap(), "nosniff");
		assert_eq!(headers.get(REFERRER_POLICY).unwrap(), "no-referrer");
	}

	#[actix_web::test]
	async fn an_error_response_is_protected_too() {
		let headers = response_headers(false, "/boom").await;
		assert!(
			headers.get(CONTENT_SECURITY_POLICY).is_some(),
			"a 500 renders in a browser like any other response"
		);
		assert!(headers.get(X_FRAME_OPTIONS).is_some());
	}

	#[actix_web::test]
	async fn hsts_is_sent_only_when_the_deployment_is_https() {
		let plain = response_headers(false, "/ok").await;
		assert!(
			plain.get(STRICT_TRANSPORT_SECURITY).is_none(),
			"pinning a plain-http dev box to https would lock the developer out of it"
		);

		let secure = response_headers(true, "/ok").await;
		assert_eq!(
			secure.get(STRICT_TRANSPORT_SECURITY).unwrap(),
			security::STRICT_TRANSPORT_SECURITY
		);
	}

	#[test]
	fn the_policy_denies_framing_and_plugins() {
		let policy = security::CONTENT_SECURITY_POLICY;
		assert!(policy.contains("frame-ancestors 'none'"));
		assert!(policy.contains("object-src 'none'"));
		assert!(policy.contains("default-src 'self'"));
		assert!(
			policy.contains("img-src 'self' https: data:"),
			"covers are served from audible and audnex hosts, so images must not be self only"
		);
		assert!(
			!policy.contains("unsafe-inline") && !policy.contains("unsafe-eval"),
			"the frontend must be built against a strict policy rather than relaxing it later"
		);
	}
}
