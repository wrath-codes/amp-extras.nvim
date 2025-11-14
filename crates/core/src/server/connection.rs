//! Per-connection WebSocket handling

use std::{
    net::TcpStream,
    time::{Duration, Instant},
};

use crossbeam_channel::unbounded;
use tungstenite::{
    accept_hdr,
    handshake::server::{Request, Response},
    Error as WsError, Message, WebSocket,
};
use url::Url;

use super::hub::Hub;
use crate::{rpc, util::ct_eq};

/// Ping interval - send ping every 30 seconds
const PING_INTERVAL: Duration = Duration::from_secs(30);

/// Timeout for detecting dead connections (60 seconds without pong)
const PONG_TIMEOUT: Duration = Duration::from_secs(60);

/// Read timeout for non-blocking reads (use short timeout to check heartbeat)
const READ_TIMEOUT: Duration = Duration::from_millis(100);

/// Write timeout for sending messages (5 seconds)
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);

/// Handle a single WebSocket connection
///
/// Performs handshake, validates auth token, and processes messages.
/// Connection lifecycle: handshake → register → message loop → unregister → notify disconnect
pub fn handle_connection(stream: TcpStream, expected_token: String, hub: Hub) {
    // Generate unique client ID
    let client_id = Hub::next_client_id();

    match accept_with_auth(stream, &expected_token) {
        Ok(websocket) => {
            // Create channel for receiving outbound messages from Hub
            let (tx, rx) = unbounded();

            // Register client with hub
            hub.register(client_id, tx);

            // Fire autocommand for client connected
            #[cfg(not(test))]
            super::events::notify_client_connected();

            // Notify user that Amp CLI connected
            #[cfg(not(test))]
            {
                let _ = crate::ide_ops::schedule_on_main_thread(|| {
                    use nvim_oxi::api;

                    let _ = api::notify(
                        "Amp CLI: Connected ",
                        api::types::LogLevel::Info,
                        &Default::default(),
                    );
                });
            }

            // Send initial state to newly connected client
            #[cfg(not(test))]
            send_initial_state(hub.clone());

            // Run message loop
            let _ = run_message_loop(websocket, rx, client_id);

            // Unregister client when done
            hub.unregister(client_id);

            // Fire disconnect event only if this was the last client
            #[cfg(not(test))]
            if hub.client_count() == 0 {
                super::events::notify_client_disconnected();
            }
        },
        Err(_e) => {
            // Handshake failed - connection will be dropped
        },
    }
}

/// Run the WebSocket message loop
///
/// Processes incoming messages until connection closes or error occurs.
/// Also checks for outbound messages from the Hub to send to the client.
///
/// Returns Ok(reason) on clean disconnect, Err on error.
fn run_message_loop(
    mut websocket: WebSocket<TcpStream>,
    outbound_rx: crossbeam_channel::Receiver<String>,
    client_id: u64,
) -> Result<String, WsError> {
    // Set read and write timeouts on the underlying stream
    websocket
        .get_mut()
        .set_read_timeout(Some(READ_TIMEOUT))
        .map_err(WsError::Io)?;
    websocket
        .get_mut()
        .set_write_timeout(Some(WRITE_TIMEOUT))
        .map_err(WsError::Io)?;

    // Heartbeat tracking
    let mut last_ping = Instant::now();
    let mut last_pong = Instant::now();

    loop {
        let now = Instant::now();

        // Check if we need to send a ping
        if now.duration_since(last_ping) >= PING_INTERVAL {
            websocket.send(Message::Ping(vec![].into()))?;
            last_ping = now;
        }

        // Check if connection is dead (no pong received)
        if now.duration_since(last_pong) >= PONG_TIMEOUT {
            break;
        }

        // Check for outbound messages from Hub (non-blocking)
        if let Ok(msg) = outbound_rx.try_recv() {
            websocket.send(Message::Text(msg.into()))?;
        }

        match websocket.read() {
            Ok(msg) => {
                let _ = handle_message(&mut websocket, msg, &mut last_pong, client_id);
            },
            Err(WsError::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // Read timeout - this is normal, just check heartbeat and continue
                continue;
            },
            Err(WsError::Io(e)) if e.kind() == std::io::ErrorKind::TimedOut => {
                // Socket timeout - check heartbeat and continue
                continue;
            },
            Err(WsError::ConnectionClosed) => {
                // Normal close
                break;
            },
            Err(_e) => {
                // Connection error
                break;
            },
        }
    }

    // Cleanup
    let _ = websocket.close(None);
    Ok("normal closure".to_string())
}

/// Handle a single WebSocket message
///
/// Routes message to appropriate handler based on type
fn handle_message(
    websocket: &mut WebSocket<TcpStream>,
    msg: Message,
    last_pong: &mut Instant,
    _client_id: u64,
) -> Result<(), WsError> {
    match msg {
        Message::Text(text) => {
            // Route to JSON-RPC handler
            handle_text_message(websocket, &text)?;
        },
        Message::Ping(data) => {
            // Respond with pong
            websocket.send(Message::Pong(data))?;
        },
        Message::Pong(_) => {
            // Pong received - update timestamp
            *last_pong = Instant::now();
        },
        Message::Close(frame) => {
            // Client initiated close - send close response and return error to exit loop
            websocket.send(Message::Close(frame))?;
            return Err(WsError::ConnectionClosed);
        },
        Message::Binary(_) => {
            // We don't support binary messages
        },
        Message::Frame(_) => {
            // Raw frames should not appear in read()
        },
    }
    Ok(())
}

/// Handle a text message (JSON-RPC)
///
/// Routes to JSON-RPC router and sends response if needed
fn handle_text_message(websocket: &mut WebSocket<TcpStream>, text: &str) -> Result<(), WsError> {
    // Route through JSON-RPC handler
    match rpc::router::handle_text(text) {
        Ok(Some(response_json)) => {
            // Send response for requests
            websocket.send(Message::Text(response_json.into()))?;
        },
        Ok(None) => {
            // Notification - no response needed
        },
        Err(_e) => {
            // Don't send error response for malformed requests
        },
    }
    Ok(())
}

/// Accept WebSocket connection with authentication
///
/// Validates auth token from query parameter and completes handshake
fn accept_with_auth(
    stream: TcpStream,
    expected_token: &str,
) -> Result<tungstenite::WebSocket<TcpStream>, WsError> {
    let callback = |req: &Request, response: Response| {
        // Parse the URI to extract query parameters
        let uri = req.uri().to_string();

        // Construct full URL for parsing (ws://host/path?query)
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
                // Extract auth parameter
                let auth_token = url
                    .query_pairs()
                    .find(|(key, _)| key == "auth")
                    .map(|(_, value)| value.into_owned());

                match auth_token {
                    Some(token) if ct_eq(&token, expected_token) => {
                        // Valid token, allow connection
                        Ok(response)
                    },
                    _ => {
                        // Invalid or missing token, return 401
                        let error_response = http::Response::builder()
                            .status(http::StatusCode::UNAUTHORIZED)
                            .body(Some("Unauthorized".to_string()))
                            .unwrap();
                        Err(error_response)
                    },
                }
            },
            Err(_) => {
                // Failed to parse URL
                let error_response = http::Response::builder()
                    .status(http::StatusCode::BAD_REQUEST)
                    .body(Some("Bad Request".to_string()))
                    .unwrap();
                Err(error_response)
            },
        }
    };

    accept_hdr(stream, callback).map_err(|e| {
        // Convert HandshakeError to WsError
        match e {
            tungstenite::handshake::HandshakeError::Interrupted(_) => WsError::Io(
                std::io::Error::new(std::io::ErrorKind::Interrupted, "Handshake interrupted"),
            ),
            tungstenite::handshake::HandshakeError::Failure(err) => err,
        }
    })
}

/// Send initial state to newly connected client
///
/// Sends plugin metadata after 200ms, then visible files and selection after
/// 250ms total. This gives the client time to be ready before receiving state.
#[cfg(not(test))]
fn send_initial_state(hub: Hub) {
    std::thread::spawn(move || {
        // Wait 200ms before sending plugin metadata
        std::thread::sleep(Duration::from_millis(200));

        // Check if still connected (hub has clients)
        if hub.client_count() == 0 {
            return;
        }

        // Send plugin metadata (no Neovim API calls needed)
        let version = env!("CARGO_PKG_VERSION");
        let plugin_dir = env!("CARGO_MANIFEST_DIR");
        let _ = crate::notifications::send_plugin_metadata(&hub, version, plugin_dir);

        // Wait additional 50ms before sending state
        std::thread::sleep(Duration::from_millis(50));

        // Check again if still connected and Neovim is available
        if hub.client_count() == 0 || !crate::ide_ops::nvim_available() {
            return;
        }

        // Schedule state collection on Neovim main thread
        let hub_clone = hub.clone();
        let _ = crate::ide_ops::schedule_on_main_thread(move || {
            // Send visible files, current selection, and diagnostics
            let _ = crate::autocmds::handle_visible_files_changed(&hub_clone);
            let _ = crate::autocmds::handle_cursor_moved(&hub_clone);
            let _ = crate::autocmds::handle_diagnostics_changed(&hub_clone);
        });
    });
}

#[cfg(test)]
mod tests {
    use std::{net::TcpListener, thread, time::Duration};

    use super::*;

    fn setup_test_server(expected_token: String) -> (u16, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let handle = thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                let _ = accept_with_auth(stream, &expected_token);
            }
        });

        thread::sleep(Duration::from_millis(50));
        (port, handle)
    }

    #[test]
    fn test_successful_handshake() {
        let token = "test_token_12345";
        let (port, handle) = setup_test_server(token.to_string());

        // Connect with valid token
        let url = format!("ws://127.0.0.1:{}/?auth={}", port, token);
        let result = tungstenite::connect(&url);

        // Should succeed
        assert!(result.is_ok());

        // Cleanup
        let _ = handle.join();
    }

    #[test]
    fn test_handshake_wrong_token() {
        let token = "correct_token";
        let (port, handle) = setup_test_server(token.to_string());

        // Connect with wrong token
        let url = format!("ws://127.0.0.1:{}/?auth=wrong_token", port);
        let result = tungstenite::connect(&url);

        // Should fail
        assert!(result.is_err());

        // Cleanup
        let _ = handle.join();
    }

    #[test]
    fn test_handshake_missing_token() {
        let token = "required_token";
        let (port, handle) = setup_test_server(token.to_string());

        // Connect without auth parameter
        let url = format!("ws://127.0.0.1:{}/", port);
        let result = tungstenite::connect(&url);

        // Should fail
        assert!(result.is_err());

        // Cleanup
        let _ = handle.join();
    }

    #[test]
    fn test_constant_time_comparison() {
        // Verify we're using constant-time comparison
        let token1 = "abcdefghijklmnop";
        let token2 = "abcdefghijklmnop";
        let token3 = "xxxxxxxxxxxxxxxx";

        assert!(ct_eq(token1, token2));
        assert!(!ct_eq(token1, token3));
    }
}
