use actix_web::{Error, HttpRequest, HttpResponse, get, web};
use actix_ws::Message;
use futures_util::StreamExt;
use serde::Serialize;
use tokio::sync::broadcast::error::RecvError;

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

#[utoipa::path(tag = "system", responses((status = 101, description = "WebSocket upgrade for live activity events")))]
#[get("/ws")]
pub async fn ws(
	req: HttpRequest,
	body: web::Payload,
	_admin: AdminUser,
	state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
	let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;
	let mut events = state.hub.subscribe();

	actix_web::rt::spawn(async move {
		loop {
			tokio::select! {
				incoming = msg_stream.next() => {
					match incoming {
						Some(Ok(Message::Ping(bytes))) => {
							if session.pong(&bytes).await.is_err() {
								break;
							}
						}
						Some(Ok(Message::Close(_))) | None => break,
						Some(Ok(_)) => {}
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
			}
		}

		let _ = session.close(None).await;
	});

	Ok(response)
}
