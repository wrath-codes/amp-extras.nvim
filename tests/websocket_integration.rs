//! WebSocket server integration tests
//!
//! These tests require Neovim context for AsyncHandle initialization.
//! Run with: just test-integration

use std::thread;
use std::time::Duration;
use tungstenite::{connect, Message};
use url::Url;
use serde_json::json;

/// Test helper: Start server and return (port, token)
///
/// Note: This is ignored in Rust tests because it requires Neovim AsyncHandle.
/// The Lua integration test in tests/server_test.lua covers this.
#[test]
#[ignore = "Requires Neovim context - run 'just test-integration'"]
fn test_server_websocket_lifecycle() {
    // This test is implemented in tests/server_test.lua
    // We keep this as a marker for documentation
}

/// Test WebSocket connection with valid auth token
///
/// This test can run standalone if server is already running
#[test]
#[ignore = "Manual test - requires running server"]
fn test_websocket_connect_with_auth() {
    let port = 12345; // Replace with actual port
    let token = "test_token"; // Replace with actual token
    
    let url = format!("ws://127.0.0.1:{}/?auth={}", port, token);
    let result = connect(Url::parse(&url).unwrap());
    
    assert!(result.is_ok(), "Should connect with valid token");
    
    if let Ok((mut socket, _)) = result {
        // Send ping request
        let ping_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "ping",
            "params": {}
        });
        
        socket.send(Message::Text(ping_request.to_string())).unwrap();
        
        // Receive response
        let response = socket.read().unwrap();
        if let Message::Text(text) = response {
            let value: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert_eq!(value["jsonrpc"], "2.0");
            assert_eq!(value["id"], 1);
            assert!(value["result"]["pong"].as_bool().unwrap());
        } else {
            panic!("Expected text message");
        }
        
        socket.close(None).unwrap();
    }
}

/// Test WebSocket connection with invalid auth token
#[test]
#[ignore = "Manual test - requires running server"]
fn test_websocket_connect_invalid_auth() {
    let port = 12345; // Replace with actual port
    let url = format!("ws://127.0.0.1:{}/?auth=invalid_token", port);
    
    let result = connect(Url::parse(&url).unwrap());
    assert!(result.is_err(), "Should reject invalid token with 401");
}

/// Test JSON-RPC ide/ping
#[test]
#[ignore = "Manual test - requires running server"]
fn test_jsonrpc_ide_ping() {
    let port = 12345;
    let token = "test_token";
    
    let url = format!("ws://127.0.0.1:{}/?auth={}", port, token);
    let (mut socket, _) = connect(Url::parse(&url).unwrap()).unwrap();
    
    // Send ide/ping request
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "ide/ping",
        "params": {}
    });
    
    socket.send(Message::Text(request.to_string())).unwrap();
    
    // Read response
    let response = socket.read().unwrap();
    if let Message::Text(text) = response {
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["jsonrpc"], "2.0");
        assert_eq!(value["id"], 1);
        assert_eq!(value["result"]["pong"], true);
        assert!(value["result"]["ts"].is_string());
    }
    
    socket.close(None).unwrap();
}

/// Test JSON-RPC ide/readFile
#[test]
#[ignore = "Manual test - requires running server"]
fn test_jsonrpc_ide_read_file() {
    use std::fs;
    use tempfile::NamedTempFile;
    
    let port = 12345;
    let token = "test_token";
    
    // Create temp file
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap();
    fs::write(path, "test content").unwrap();
    
    let url = format!("ws://127.0.0.1:{}/?auth={}", port, token);
    let (mut socket, _) = connect(Url::parse(&url).unwrap()).unwrap();
    
    // Send ide/readFile request
    let request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "ide/readFile",
        "params": {
            "path": path
        }
    });
    
    socket.send(Message::Text(request.to_string())).unwrap();
    
    // Read response
    let response = socket.read().unwrap();
    if let Message::Text(text) = response {
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["jsonrpc"], "2.0");
        assert_eq!(value["id"], 2);
        assert_eq!(value["result"]["content"], "test content");
    }
    
    socket.close(None).unwrap();
}

/// Test WebSocket heartbeat (ping/pong)
#[test]
#[ignore = "Manual test - requires running server"]
fn test_websocket_heartbeat() {
    let port = 12345;
    let token = "test_token";
    
    let url = format!("ws://127.0.0.1:{}/?auth={}", port, token);
    let (mut socket, _) = connect(Url::parse(&url).unwrap()).unwrap();
    
    // Wait for server to send ping (30s interval, but we'll wait 35s to be sure)
    socket.get_mut().set_read_timeout(Some(Duration::from_secs(35))).unwrap();
    
    // We should receive a ping from the server
    let mut received_ping = false;
    for _ in 0..3 {
        if let Ok(msg) = socket.read() {
            if let Message::Ping(_) = msg {
                received_ping = true;
                break;
            }
        }
    }
    
    assert!(received_ping, "Should receive ping from server");
    socket.close(None).unwrap();
}

/// Test notification (no response expected)
#[test]
#[ignore = "Manual test - requires running server"]
fn test_jsonrpc_notification() {
    let port = 12345;
    let token = "test_token";
    
    let url = format!("ws://127.0.0.1:{}/?auth={}", port, token);
    let (mut socket, _) = connect(Url::parse(&url).unwrap()).unwrap();
    
    // Send notification (no id field)
    let notification = json!({
        "jsonrpc": "2.0",
        "method": "ping",
        "params": {}
    });
    
    socket.send(Message::Text(notification.to_string())).unwrap();
    
    // Set short timeout - should not receive response
    socket.get_mut().set_read_timeout(Some(Duration::from_millis(500))).unwrap();
    
    // Try to read - should timeout (no response for notification)
    let result = socket.read();
    assert!(
        result.is_err() || matches!(result.unwrap(), Message::Ping(_)),
        "Should not receive response for notification"
    );
    
    socket.close(None).unwrap();
}

/// Test multiple concurrent clients
#[test]
#[ignore = "Manual test - requires running server"]
fn test_multiple_clients() {
    let port = 12345;
    let token = "test_token";
    
    let url = format!("ws://127.0.0.1:{}/?auth={}", port, token);
    
    // Connect 3 clients
    let mut clients = vec![];
    for i in 0..3 {
        let (socket, _) = connect(Url::parse(&url).unwrap())
            .expect(&format!("Client {} should connect", i));
        clients.push(socket);
    }
    
    // Each client sends a request
    for (i, client) in clients.iter_mut().enumerate() {
        let request = json!({
            "jsonrpc": "2.0",
            "id": i,
            "method": "ping",
            "params": {}
        });
        
        client.send(Message::Text(request.to_string())).unwrap();
        
        let response = client.read().unwrap();
        if let Message::Text(text) = response {
            let value: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert_eq!(value["id"], i);
        }
    }
    
    // Close all clients
    for mut client in clients {
        client.close(None).unwrap();
    }
}
