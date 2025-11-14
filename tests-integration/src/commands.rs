//! Integration tests for command dispatch and handlers

use amp_extras_core as amp_extras;
use nvim_oxi::api;
use serde_json::json;

#[nvim_oxi::test]
fn test_ping_command() {
    let args = json!({"message": "hello"});
    let result = amp_extras::commands::dispatch("ping", args).unwrap();

    assert_eq!(result["pong"], json!(true));
    assert_eq!(result["message"], json!("hello"));
}

#[nvim_oxi::test]
fn test_send_file_ref() {
    // Start WebSocket server for testing
    let _ = amp_extras::server::start();

    // Create a buffer with a known path
    let mut buf = api::create_buf(true, false).unwrap();
    api::set_current_buf(&buf).unwrap();
    buf.set_name("/tmp/test.rs").unwrap();

    let result = amp_extras::commands::dispatch("send_file_ref", json!({})).unwrap();

    assert_eq!(result["success"], json!(true));
    let reference = result["reference"].as_str().unwrap();
    assert!(reference.starts_with("@"));
    assert!(reference.contains("test.rs"));

    // Stop server
    amp_extras::server::stop();
}

#[nvim_oxi::test]
fn test_send_line_ref() {
    // Start WebSocket server for testing
    let _ = amp_extras::server::start();

    // Create buffer and set cursor position
    let mut buf = api::create_buf(true, false).unwrap();
    api::set_current_buf(&buf).unwrap();
    buf.set_name("/tmp/test.rs").unwrap();
    buf.set_lines(.., false, ["line 1", "line 2", "line 3"])
        .unwrap();

    // Set cursor to line 2 (1-indexed)
    let mut win = api::get_current_win();
    win.set_cursor(2, 0).unwrap();

    let result = amp_extras::commands::dispatch("send_line_ref", json!({})).unwrap();

    assert_eq!(result["success"], json!(true));
    let reference = result["reference"].as_str().unwrap();
    assert!(reference.contains("#L2"));

    // Stop server
    amp_extras::server::stop();
}

#[nvim_oxi::test]
fn test_send_buffer() {
    // Start WebSocket server for testing
    let _ = amp_extras::server::start();

    // Create buffer with content
    let mut buf = api::create_buf(true, false).unwrap();
    api::set_current_buf(&buf).unwrap();
    buf.set_name("/tmp/test.rs").unwrap();
    buf.set_lines(.., false, ["fn main() {", "    println!(\"test\");", "}"])
        .unwrap();

    let result = amp_extras::commands::dispatch("send_buffer", json!({})).unwrap();

    assert_eq!(result["success"], json!(true));
    // send_buffer just returns success - content is sent to Amp via notification

    // Stop server
    amp_extras::server::stop();
}

#[nvim_oxi::test]
fn test_send_selection() {
    // Start WebSocket server for testing
    let _ = amp_extras::server::start();

    // Create buffer with content
    let mut buf = api::create_buf(true, false).unwrap();
    api::set_current_buf(&buf).unwrap();
    buf.set_name("/tmp/test-selection.rs").unwrap();
    buf.set_lines(.., false, ["line 1", "line 2", "line 3", "line 4"])
        .unwrap();

    // Send lines 1-2 (1-indexed)
    let result = amp_extras::commands::dispatch(
        "send_selection",
        json!({
            "start_line": 1,
            "end_line": 2
        }),
    )
    .unwrap();

    assert_eq!(result["success"], json!(true));

    // Stop server
    amp_extras::server::stop();
}

#[nvim_oxi::test]
fn test_send_selection_ref() {
    // Start WebSocket server for testing
    let _ = amp_extras::server::start();

    // Create buffer with content
    let mut buf = api::create_buf(true, false).unwrap();
    api::set_current_buf(&buf).unwrap();
    buf.set_name("/tmp/test.rs").unwrap();
    buf.set_lines(.., false, ["line 1", "line 2", "line 3", "line 4"])
        .unwrap();

    // Send reference for lines 2-3 (1-indexed)
    let result = amp_extras::commands::dispatch(
        "send_selection_ref",
        json!({
            "start_line": 2,
            "end_line": 3
        }),
    )
    .unwrap();

    assert_eq!(result["success"], json!(true));
    let reference = result["reference"].as_str().unwrap();
    assert!(reference.contains("#L2-L3") || reference.contains("L2"));

    // Stop server
    amp_extras::server::stop();
}

#[nvim_oxi::test]
fn test_command_not_found() {
    let result = amp_extras::commands::dispatch("nonexistent_command", json!({}));
    assert!(result.is_err());
}

#[nvim_oxi::test]
fn test_list_commands() {
    let commands = amp_extras::commands::list_commands();

    assert!(!commands.is_empty());
    assert!(commands.contains(&"ping".to_string()));
    assert!(commands.contains(&"send_file_ref".to_string()));
    assert!(commands.contains(&"send_buffer".to_string()));
}
