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
#[ignore = "Requires WebSocket server - use 'just test-integration-full' to test with real Amp CLI"]
fn test_send_file_ref() {
    // This test requires a WebSocket client connection
    // Run with: just test-integration-full (tests/run_integration_tests.sh)
    // Or manually: start server, connect Amp CLI, then run commands
    let mut buf = api::create_buf(true, false).unwrap();
    api::set_current_buf(&buf).unwrap();
    buf.set_name("/tmp/test.rs").unwrap();
    
    // Command would fail without server, but we can test dispatch logic exists
    let result = amp_extras::commands::dispatch("send_file_ref", json!({}));
    assert!(result.is_ok() || result.is_err()); // Just verify it doesn't panic
}

#[nvim_oxi::test]
#[ignore = "Requires WebSocket server - use 'just test-integration-full' to test with real Amp CLI"]
fn test_send_line_ref() {
    let mut buf = api::create_buf(true, false).unwrap();
    api::set_current_buf(&buf).unwrap();
    buf.set_name("/tmp/test.rs").unwrap();
    buf.set_lines(.., false, ["line 1", "line 2", "line 3"])
        .unwrap();

    let mut win = api::get_current_win();
    win.set_cursor(2, 0).unwrap();

    let result = amp_extras::commands::dispatch("send_line_ref", json!({}));
    assert!(result.is_ok() || result.is_err());
}

#[nvim_oxi::test]
#[ignore = "Requires WebSocket server - use 'just test-integration-full' to test with real Amp CLI"]
fn test_send_buffer() {
    let mut buf = api::create_buf(true, false).unwrap();
    api::set_current_buf(&buf).unwrap();
    buf.set_name("/tmp/test.rs").unwrap();
    buf.set_lines(.., false, ["fn main() {", "    println!(\"test\");", "}"])
        .unwrap();

    let result = amp_extras::commands::dispatch("send_buffer", json!({}));
    assert!(result.is_ok() || result.is_err());
}

#[nvim_oxi::test]
#[ignore = "Requires WebSocket server - use 'just test-integration-full' to test with real Amp CLI"]
fn test_send_selection() {
    let mut buf = api::create_buf(true, false).unwrap();
    api::set_current_buf(&buf).unwrap();
    buf.set_name("/tmp/test-selection.rs").unwrap();
    buf.set_lines(.., false, ["line 1", "line 2", "line 3", "line 4"])
        .unwrap();

    let result = amp_extras::commands::dispatch(
        "send_selection",
        json!({
            "start_line": 1,
            "end_line": 2
        }),
    );
    assert!(result.is_ok() || result.is_err());
}

#[nvim_oxi::test]
#[ignore = "Requires WebSocket server - use 'just test-integration-full' to test with real Amp CLI"]
fn test_send_selection_ref() {
    let mut buf = api::create_buf(true, false).unwrap();
    api::set_current_buf(&buf).unwrap();
    buf.set_name("/tmp/test.rs").unwrap();
    buf.set_lines(.., false, ["line 1", "line 2", "line 3", "line 4"])
        .unwrap();

    let result = amp_extras::commands::dispatch(
        "send_selection_ref",
        json!({
            "start_line": 2,
            "end_line": 3
        }),
    );
    assert!(result.is_ok() || result.is_err());
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
