#!/usr/bin/env -S cargo +stable test --quiet --test
//! WebSocket test client to verify notifications
//!
//! Usage:
//!   1. Start Neovim with the plugin loaded and server running
//!   2. Run: cargo test --test websocket_client -- --nocapture
//!   3. In Neovim: move cursor, make visual selections, open files
//!   4. Watch this client receive and print notifications

use std::time::Duration;

use tokio_tungstenite::{connect_async, tungstenite::Message};

#[tokio::test]
async fn test_websocket_client() {
    println!("\n=== WebSocket Notification Test Client ===");
    println!("This test connects to a running WebSocket server and prints received notifications.");
    println!();
    println!("Instructions:");
    println!("  1. Start Neovim in another terminal");
    println!("  2. Run: :lua require('amp_extras').server_start()");
    println!("  3. Note the port and token from the output");
    println!("  4. Export them: export WS_PORT=<port> WS_TOKEN=<token>");
    println!("  5. Run this test");
    println!();

    // Get port and token from environment
    let port = std::env::var("WS_PORT")
        .expect("Set WS_PORT environment variable (from server_start output)");
    let token = std::env::var("WS_TOKEN")
        .expect("Set WS_TOKEN environment variable (from server_start output)");

    println!(
        "Connecting to ws://127.0.0.1:{}/?auth={}",
        port,
        &token[..8]
    );

    // Connect to WebSocket server
    let url = format!("ws://127.0.0.1:{}/?auth={}", port, token);

    let (ws_stream, response) = connect_async(url).await.expect("Failed to connect - is the server running?");

    println!("Connected! HTTP Status: {}", response.status());
    println!();
    println!("Waiting for notifications... (Ctrl+C to stop)");
    println!("In Neovim:");
    println!("  - Move cursor around");
    println!("  - Enter visual mode (v, V, Ctrl-V) and select text");
    println!("  - Open new files / splits");
    println!();
    println!("─────────────────────────────────────────────────────");
    println!();

    // Note: set_read_timeout not available on MaybeTlsStream
    // Will use non-blocking read with timeout handling instead

    use futures_util::StreamExt;
    
    let (_, mut read) = ws_stream.split();
    
    let mut message_count = 0;
    let start_time = std::time::Instant::now();
    let timeout = Duration::from_secs(5); // Wait up to 5 seconds for messages

    // Read messages for 5 seconds or until error
    while let Some(msg_result) = tokio::time::timeout(Duration::from_millis(100), read.next()).await.ok().flatten() {
        if start_time.elapsed() > timeout && message_count == 0 {
            println!("⏱️  No messages received in {} seconds", timeout.as_secs());
            break;
        }

        match msg_result {
            Ok(msg) => {
                message_count += 1;

                match msg {
                    Message::Text(text) => {
                        println!("📨 Message #{}", message_count);

                        // Pretty print JSON
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                            println!("{}", serde_json::to_string_pretty(&json).unwrap());
                        } else {
                            println!("{}", text);
                        }

                        println!();
                    },
                    Message::Ping(_) => {
                        println!("🏓 Received ping");
                    },
                    Message::Pong(_) => {
                        println!("🏓 Received pong");
                    },
                    Message::Close(_) => {
                        println!("👋 Server closed connection");
                        break;
                    },
                    _ => {},
                }
            },
            Err(e) => {
                eprintln!("❌ Error: {}", e);
                break;
            },
        }
    }

    println!();
    println!("─────────────────────────────────────────────────────");
    println!("Total messages received: {}", message_count);
    println!();

    // Connection will be dropped automatically
}
