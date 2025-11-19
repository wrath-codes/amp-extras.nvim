//! Integration tests for command dispatch and handlers

use amp_extras_core as amp_extras;
use serde_json::json;

#[nvim_oxi::test]
fn test_ping_command() {
    let args = json!({"message": "hello"});
    let result = amp_extras::commands::dispatch("ping", args).unwrap();

    assert_eq!(result["pong"], json!(true));
    assert_eq!(result["message"], json!("hello"));
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
    // Deleted commands should not be present
    assert!(!commands.contains(&"send_file_ref".to_string()));
    assert!(!commands.contains(&"send_buffer".to_string()));
}
