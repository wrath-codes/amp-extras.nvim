//! Integration tests for WebSocket server
//!
//! These tests verify:
//! - Authentication failures (401)
//! - Multiple concurrent clients
//! - Connection timeouts

use std::time::Duration;
use tungstenite::{connect, Message, Error as WsError};

#[test]
fn test_auth_failure_wrong_token() {
    println!("\n=== Test: Authentication Failure - Wrong Token ===");
    
    // Get server info from environment
    let port = std::env::var("WS_PORT")
        .expect("Set WS_PORT environment variable");
    let _token = std::env::var("WS_TOKEN")
        .expect("Set WS_TOKEN environment variable");
    
    // Connect with WRONG token
    let wrong_token = "wrong_token_12345";
    let url = format!("ws://127.0.0.1:{}/?auth={}", port, wrong_token);
    
    println!("Attempting to connect with wrong token...");
    let result = connect(url);
    
    match result {
        Ok(_) => {
            panic!("Connection should have failed with wrong token!");
        }
        Err(WsError::Http(response)) => {
            println!("✅ Connection rejected with HTTP status: {}", response.status());
            assert_eq!(response.status(), 401, "Should return 401 Unauthorized");
        }
        Err(e) => {
            println!("✅ Connection failed as expected: {}", e);
        }
    }
}

#[test]
fn test_auth_failure_missing_token() {
    println!("\n=== Test: Authentication Failure - Missing Token ===");
    
    let port = std::env::var("WS_PORT")
        .expect("Set WS_PORT environment variable");
    
    // Connect WITHOUT auth parameter
    let url = format!("ws://127.0.0.1:{}/", port);
    
    println!("Attempting to connect without token...");
    let result = connect(url);
    
    match result {
        Ok(_) => {
            panic!("Connection should have failed without token!");
        }
        Err(WsError::Http(response)) => {
            println!("✅ Connection rejected with HTTP status: {}", response.status());
            assert_eq!(response.status(), 401, "Should return 401 Unauthorized");
        }
        Err(e) => {
            println!("✅ Connection failed as expected: {}", e);
        }
    }
}

#[test]
fn test_multiple_concurrent_clients() {
    println!("\n=== Test: Multiple Concurrent Clients ===");
    
    let port = std::env::var("WS_PORT")
        .expect("Set WS_PORT environment variable");
    let token = std::env::var("WS_TOKEN")
        .expect("Set WS_TOKEN environment variable");
    
    let url = format!("ws://127.0.0.1:{}/?auth={}", port, token);
    
    // Connect first client
    println!("Connecting client 1...");
    let (mut client1, _) = connect(&url)
        .expect("Client 1 should connect");
    println!("✅ Client 1 connected");
    
    // Connect second client
    println!("Connecting client 2...");
    let (mut client2, _) = connect(&url)
        .expect("Client 2 should connect");
    println!("✅ Client 2 connected");
    
    // Connect third client
    println!("Connecting client 3...");
    let (mut client3, _) = connect(&url)
        .expect("Client 3 should connect");
    println!("✅ Client 3 connected");
    
    println!("\n✅ All 3 clients connected successfully!");
    
    // Send a ping from client 1
    println!("\nSending ping from client 1...");
    let ping_request = r#"{"jsonrpc":"2.0","id":1,"method":"ping","params":{}}"#;
    client1.send(Message::Text(ping_request.to_string().into()))
        .expect("Should send ping");
    
    // Read response
    match client1.read() {
        Ok(Message::Text(text)) => {
            println!("Client 1 received response: {}", text);
            assert!(text.contains("pong"), "Response should contain 'pong'");
        }
        Ok(msg) => println!("Client 1 received: {:?}", msg),
        Err(e) => println!("Client 1 read error: {}", e),
    }
    
    // Verify all clients can still read (non-blocking check)
    println!("\nVerifying all clients are still connected...");
    
    // Send ping to client 2
    client2.send(Message::Ping(vec![].into()))
        .expect("Client 2 should send ping");
    
    // Send ping to client 3
    client3.send(Message::Ping(vec![].into()))
        .expect("Client 3 should send ping");
    
    println!("✅ All clients can send messages");
    
    // Close connections
    println!("\nClosing connections...");
    let _ = client1.close(None);
    let _ = client2.close(None);
    let _ = client3.close(None);
    
    println!("✅ Multiple concurrent clients test passed!");
}

#[test]
#[ignore = "This test takes 60+ seconds to run"]
fn test_connection_timeout() {
    println!("\n=== Test: Connection Timeout (Heartbeat) ===");
    println!("NOTE: This test takes 60+ seconds to run");
    println!("The server sends ping every 30s and times out after 60s without pong");
    
    let port = std::env::var("WS_PORT")
        .expect("Set WS_PORT environment variable");
    let token = std::env::var("WS_TOKEN")
        .expect("Set WS_TOKEN environment variable");
    
    let url = format!("ws://127.0.0.1:{}/?auth={}", port, token);
    
    println!("Connecting to server...");
    let (mut socket, _) = connect(url)
        .expect("Should connect");
    println!("✅ Connected");
    
    println!("\nIgnoring all ping messages (no pong responses)...");
    println!("Waiting for server to timeout (60 seconds)...");
    
    let start = std::time::Instant::now();
    let mut received_ping = false;
    
    loop {
        match socket.read() {
            Ok(Message::Ping(_)) => {
                received_ping = true;
                println!("🏓 Received ping at {:?} (NOT sending pong)", start.elapsed());
                // Intentionally NOT sending pong to trigger timeout
            }
            Ok(Message::Close(_)) => {
                let elapsed = start.elapsed();
                println!("👋 Server closed connection after {:?}", elapsed);
                assert!(received_ping, "Should have received at least one ping");
                assert!(elapsed > Duration::from_secs(55), "Should timeout after ~60s");
                break;
            }
            Ok(msg) => {
                println!("Received: {:?}", msg);
            }
            Err(WsError::ConnectionClosed) => {
                let elapsed = start.elapsed();
                println!("✅ Connection closed by server after {:?}", elapsed);
                assert!(received_ping, "Should have received at least one ping");
                break;
            }
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
        }
        
        // Safety timeout
        if start.elapsed() > Duration::from_secs(90) {
            panic!("Test timeout - server should have closed connection");
        }
    }
    
    println!("✅ Connection timeout test passed!");
}

#[test]
fn test_ping_pong_exchange() {
    println!("\n=== Test: Ping-Pong Exchange ===");
    
    let port = std::env::var("WS_PORT")
        .expect("Set WS_PORT environment variable");
    let token = std::env::var("WS_TOKEN")
        .expect("Set WS_TOKEN environment variable");
    
    let url = format!("ws://127.0.0.1:{}/?auth={}", port, token);
    
    println!("Connecting to server...");
    let (mut socket, _) = connect(url)
        .expect("Should connect");
    println!("✅ Connected");
    
    // Send a ping
    println!("Sending ping to server...");
    socket.send(Message::Ping(vec![1, 2, 3, 4].into()))
        .expect("Should send ping");
    
    // Wait for pong
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(5);
    
    loop {
        match socket.read() {
            Ok(Message::Pong(data)) => {
                println!("✅ Received pong: {:?}", data);
                assert_eq!(data, vec![1, 2, 3, 4], "Pong data should match ping data");
                break;
            }
            Ok(msg) => {
                println!("Received other message: {:?}", msg);
            }
            Err(e) => {
                panic!("Error reading: {}", e);
            }
        }
        
        if start.elapsed() > timeout {
            panic!("Timeout waiting for pong");
        }
    }
    
    println!("✅ Ping-pong exchange successful!");
}
