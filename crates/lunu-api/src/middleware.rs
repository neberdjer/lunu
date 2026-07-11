use actix_web::body::{EitherBody, MessageBody};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::http::header::CONTENT_TYPE;
use actix_web::middleware::Next;

use crate::error::status_error_response;

pub async fn normalize_errors<B>(
	req: ServiceRequest,
	next: Next<B>,
) -> Result<ServiceResponse<EitherBody<B>>, actix_web::Error>
where
	B: MessageBody + 'static,
{
	let response = next.call(req).await?;
	let status = response.status();

	if status.is_client_error() || status.is_server_error() {
		let already_json = response
			.headers()
			.get(CONTENT_TYPE)
			.and_then(|value| value.to_str().ok())
			.is_some_and(|value| value.starts_with("application/json"));

		if !already_json {
			let replacement = status_error_response(status);
			return Ok(response.into_response(replacement).map_into_right_body());
		}
	}

	Ok(response.map_into_left_body())
}

#[cfg(test)]
mod tests {
	use actix_web::middleware::from_fn;
	use actix_web::{App, HttpResponse, test, web};
	use serde_json::json;

	use super::*;

	#[actix_web::test]
	async fn malformed_json_is_enveloped() {
		#[derive(serde::Deserialize)]
		struct Body {}

		let app = test::init_service(App::new().wrap(from_fn(normalize_errors)).route(
			"/echo",
			web::post().to(|_: web::Json<Body>| async { HttpResponse::Ok().finish() }),
		))
		.await;
		let request = test::TestRequest::post()
			.uri("/echo")
			.insert_header(("content-type", "application/json"))
			.set_payload("{ not valid json")
			.to_request();
		let response = test::call_service(&app, request).await;

		assert_eq!(response.status(), 400);
		let body: serde_json::Value = test::read_body_json(response).await;
		assert_eq!(body["error"]["code"], "bad_request");
	}

	#[actix_web::test]
	async fn unmatched_route_is_enveloped() {
		let app = test::init_service(App::new().wrap(from_fn(normalize_errors))).await;
		let response =
			test::call_service(&app, test::TestRequest::get().uri("/nope").to_request()).await;

		assert_eq!(response.status(), 404);
		let body: serde_json::Value = test::read_body_json(response).await;
		assert_eq!(body["error"]["code"], "not_found");
	}

	#[actix_web::test]
	async fn bare_error_response_is_enveloped() {
		let app = test::init_service(App::new().wrap(from_fn(normalize_errors)).route(
			"/boom",
			web::get().to(|| async { HttpResponse::InternalServerError().body("boom") }),
		))
		.await;
		let response =
			test::call_service(&app, test::TestRequest::get().uri("/boom").to_request()).await;

		assert_eq!(response.status(), 500);
		let body: serde_json::Value = test::read_body_json(response).await;
		assert_eq!(body["error"]["code"], "internal");
	}

	#[actix_web::test]
	async fn json_error_response_is_not_rewrapped() {
		let app = test::init_service(App::new().wrap(from_fn(normalize_errors)).route(
			"/e",
			web::get().to(|| async {
				HttpResponse::UnprocessableEntity()
					.json(json!({ "error": { "code": "validation" } }))
			}),
		))
		.await;
		let response =
			test::call_service(&app, test::TestRequest::get().uri("/e").to_request()).await;

		assert_eq!(response.status(), 422);
		let body: serde_json::Value = test::read_body_json(response).await;
		assert_eq!(body["error"]["code"], "validation");
	}

	#[actix_web::test]
	async fn successful_response_is_untouched() {
		let app = test::init_service(App::new().wrap(from_fn(normalize_errors)).route(
			"/health",
			web::get().to(|| async { HttpResponse::Ok().body("ok") }),
		))
		.await;
		let response =
			test::call_service(&app, test::TestRequest::get().uri("/health").to_request()).await;

		assert_eq!(response.status(), 200);
		assert_eq!(test::read_body(response).await, "ok");
	}
}
