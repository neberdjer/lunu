use actix_web::{Error, HttpRequest, HttpResponse, web};
use actix_ws::Message;
use futures_util::StreamExt;
use tokio::sync::broadcast::error::RecvError;

use crate::dto::ActivityResponse;
use crate::extract::AdminUser;
use crate::state::AppState;

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
						Ok(activity) => {
							let payload = serde_json::to_string(&ActivityResponse::from(&activity))
								.unwrap_or_default();
							if session.text(payload).await.is_err() {
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
