//! Async WebSocket connection handling with tokio-tungstenite

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio::time::{interval, Instant};
use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::{
        handshake::server::{Request, Response},
        Error as WsError, Message,
    },
};
use url::Url;

use super::hub::Hub;
use crate::util::ct_eq;

const PING_INTERVAL: Duration = Duration::from_secs(30);
const PONG_TIMEOUT: Duration = Duration::from_secs(60);

/// Handle a single WebSocket connection asynchronously
pub async fn handle_connection(
    stream: tokio::net::TcpStream,
    expected_token: String,
    hub: Hub,
) {
    let client_id = Hub::next_client_id();

    match accept_with_auth(stream, &expected_token).await {
        Ok(websocket) => {
            let (tx, rx) = mpsc::unbounded_channel();

            hub.register(client_id, tx);

            #[cfg(not(test))]
            {
                // Queue event for main thread processing (safe from Tokio thread)
                super::event_bridge::send_event(super::event_bridge::ServerEvent::ClientConnected);
                
                // Queue initial state send
                if crate::ide_ops::nvim_available() && super::is_running() {
                    super::event_bridge::send_event(
                        super::event_bridge::ServerEvent::SendInitialState(hub.clone())
                    );
                }
            }

            let _ = run_message_loop(websocket, rx, client_id, hub.clone()).await;

            hub.unregister(client_id);

            #[cfg(not(test))]
            if hub.client_count() == 0 {
                // Queue event for main thread processing (safe from Tokio thread)
                super::event_bridge::send_event(super::event_bridge::ServerEvent::ClientDisconnected);
            }
        }
        Err(_e) => {
            // Handshake failed
        }
    }
}

/// Run the async WebSocket message loop
async fn run_message_loop(
    websocket: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    mut outbound_rx: UnboundedReceiver<String>,
    _client_id: u64,
    _hub: Hub,
) -> Result<String, WsError> {
    let (mut write, mut read) = websocket.split();

    let mut ping_interval = interval(PING_INTERVAL);
    let mut last_pong = Instant::now();

    loop {
        // Check shutdown
        if !super::is_running() {
            let _ = write.close().await;
            return Ok("server shutdown".to_string());
        }

        // Check pong timeout
        if last_pong.elapsed() >= PONG_TIMEOUT {
            break;
        }

        tokio::select! {
            // Send ping
            _ = ping_interval.tick() => {
                if let Err(e) = write.send(Message::Ping(vec![].into())).await {
                    return Err(e);
                }
            }

            // Send outbound messages
            Some(msg) = outbound_rx.recv() => {
                if let Err(e) = write.send(Message::Text(msg.into())).await {
                    return Err(e);
                }
            }

            // Receive messages
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let _ = handle_text_message(&text).await;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = write.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Pong(_))) => {
                        last_pong = Instant::now();
                    }
                    Some(Ok(Message::Close(frame))) => {
                        let _ = write.send(Message::Close(frame)).await;
                        break;
                    }
                    Some(Err(_)) | None => {
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    let _ = write.close().await;
    Ok("normal closure".to_string())
}

/// Handle text message (JSON-RPC)
async fn handle_text_message(text: &str) -> Result<(), WsError> {
    // Route through JSON-RPC handler (still synchronous, that's OK)
    match crate::rpc::router::handle_text(text) {
        Ok(Some(_response_json)) => {
            // Note: We'd need to pass write handle here to send response
            // For now, responses are handled elsewhere
        }
        Ok(None) => {
            // Notification - no response
        }
        Err(_e) => {
            // Don't send error for malformed requests
        }
    }
    Ok(())
}

/// Accept WebSocket with authentication
async fn accept_with_auth(
    stream: tokio::net::TcpStream,
    expected_token: &str,
) -> Result<tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>, WsError> {
    let expected = expected_token.to_string();

    let callback = move |req: &Request, response: Response| {
        let uri = req.uri().to_string();
        let full_url = format!(
            "ws://{}{}",
            req.uri()
                .authority()
                .map(|a| a.as_str())
                .unwrap_or("localhost"),
            uri
        );

        match Url::parse(&full_url) {
            Ok(url) => {
                let auth_token = url
                    .query_pairs()
                    .find(|(key, _)| key == "auth")
                    .map(|(_, value)| value.into_owned());

                match auth_token {
                    Some(token) if ct_eq(&token, &expected) => Ok(response),
                    _ => {
                        let error_response = http::Response::builder()
                            .status(http::StatusCode::UNAUTHORIZED)
                            .body(Some("Unauthorized".to_string()))
                            .unwrap();
                        Err(error_response)
                    }
                }
            }
            Err(_) => {
                let error_response = http::Response::builder()
                    .status(http::StatusCode::BAD_REQUEST)
                    .body(Some("Bad Request".to_string()))
                    .unwrap();
                Err(error_response)
            }
        }
    };

    accept_hdr_async(stream, callback)
        .await
        .map_err(|e| match e {
            tokio_tungstenite::tungstenite::Error::ConnectionClosed => WsError::ConnectionClosed,
            e => WsError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)),
        })
}

// send_initial_state moved to event_bridge.rs
