use std::time::{Duration, Instant};

use actix_web::http::header::{HOST, ORIGIN};
use actix_web::{Error, HttpRequest, HttpResponse, get, web};
use actix_ws::Message;
use futures_util::StreamExt;
use serde::Serialize;
use tokio::sync::broadcast::error::RecvError;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(20);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(60);

use lunu_core::models::LiveEvent;

use crate::dto::{ActivityResponse, DownloadResponse, MediaResponse, NotificationResponse};
use crate::extract::AdminUser;
use crate::state::AppState;

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum WsMessage {
	Activity(ActivityResponse),
	Progress(DownloadResponse),
	Notification(NotificationResponse),
	Merge(MediaResponse),
}

fn encode(event: &LiveEvent) -> String {
	let message = match event {
		LiveEvent::Activity(activity) => WsMessage::Activity(ActivityResponse::from(activity)),
		LiveEvent::Progress(download) => WsMessage::Progress(DownloadResponse::from(download)),
		LiveEvent::Notification(notification) => {
			WsMessage::Notification(NotificationResponse::from(notification))
		}
		LiveEvent::Merge(media) => WsMessage::Merge(MediaResponse::from(media.as_ref().clone())),
	};
	serde_json::to_string(&message).unwrap_or_default()
}

fn same_origin(req: &HttpRequest) -> bool {
	let Some(origin) = req
		.headers()
		.get(ORIGIN)
		.and_then(|value| value.to_str().ok())
	else {
		return true;
	};
	let origin_host = origin.split_once("://").map_or(origin, |(_, rest)| rest);
	let host = req
		.headers()
		.get(HOST)
		.and_then(|value| value.to_str().ok())
		.unwrap_or_default();
	origin_host.eq_ignore_ascii_case(host)
}

#[utoipa::path(tag = "system", responses((status = 101, description = "WebSocket upgrade for live activity events")))]
#[get("/ws")]
pub async fn ws(
	req: HttpRequest,
	body: web::Payload,
	_admin: AdminUser,
	state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
	if !same_origin(&req) {
		return Ok(HttpResponse::Forbidden().finish());
	}
	let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;
	let mut events = state.hub.subscribe();

	actix_web::rt::spawn(async move {
		let mut last_seen = Instant::now();
		let mut heartbeat = tokio::time::interval(HEARTBEAT_INTERVAL);
		loop {
			tokio::select! {
				incoming = msg_stream.next() => {
					match incoming {
						Some(Ok(Message::Ping(bytes))) => {
							last_seen = Instant::now();
							if session.pong(&bytes).await.is_err() {
								break;
							}
						}
						Some(Ok(Message::Close(_))) | None => break,
						Some(Ok(_)) => last_seen = Instant::now(),
						Some(Err(_)) => break,
					}
				}
				event = events.recv() => {
					match event {
						Ok(event) => {
							if session.text(encode(&event)).await.is_err() {
								break;
							}
						}
						Err(RecvError::Lagged(_)) => {}
						Err(RecvError::Closed) => break,
					}
				}
				_ = heartbeat.tick() => {
					if last_seen.elapsed() > CLIENT_TIMEOUT || session.ping(&[]).await.is_err() {
						break;
					}
				}
			}
		}

		let _ = session.close(None).await;
	});

	Ok(response)
}
