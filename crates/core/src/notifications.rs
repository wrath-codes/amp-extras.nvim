//! Server-initiated notifications to Amp CLI
//!
//! This module handles sending notifications from Neovim to connected
//! Amp CLI clients, including:
//! - pluginMetadata - Plugin version and info
//! - selectionDidChange - Cursor/selection changes
//! - visibleFilesDidChange - Open files in windows
//! - userSentMessage - User-triggered message to agent
//! - appendToPrompt - Append text to IDE prompt field

use serde_json::json;

use crate::{
    errors::{AmpError, Result},
    rpc::ServerNotification,
    server::Hub,
};

/// Send pluginMetadata notification to all clients
///
/// Sends plugin version and directory information.
/// Typically sent once when a client connects.
pub fn send_plugin_metadata(hub: &Hub, version: &str, plugin_dir: &str) -> Result<()> {
    let notification = ServerNotification::new(
        "pluginMetadata".to_string(),
        json!({
            "version": version,
            "pluginDirectory": plugin_dir
        }),
    );

    let json = serde_json::to_string(&notification).map_err(|e| {
        AmpError::NotificationError(format!("Failed to serialize pluginMetadata: {}", e))
    })?;
    hub.broadcast(&json);
    Ok(())
}

/// Send selectionDidChange notification
///
/// Notifies clients about cursor position and selection changes.
///
/// Parameters:
/// - uri: File URI (e.g., "file:///path/to/file.txt")
/// - start_line, start_char: Selection start position (0-indexed)
/// - end_line, end_char: Selection end position (0-indexed)
/// - content: Selected text content
pub fn send_selection_changed(
    hub: &Hub,
    uri: &str,
    start_line: usize,
    start_char: usize,
    end_line: usize,
    end_char: usize,
    content: &str,
) -> Result<()> {
    let notification = ServerNotification::new(
        "selectionDidChange".to_string(),
        json!({
            "uri": uri,
            "selections": [{
                "range": {
                    "startLine": start_line,
                    "startCharacter": start_char,
                    "endLine": end_line,
                    "endCharacter": end_char
                },
                "content": content
            }]
        }),
    );

    let json = serde_json::to_string(&notification).map_err(|e| {
        AmpError::NotificationError(format!("Failed to serialize selectionDidChange: {}", e))
    })?;
    hub.broadcast(&json);
    Ok(())
}

/// Send visibleFilesDidChange notification
///
/// Notifies clients about which files are currently visible in windows.
///
/// Parameters:
/// - uris: List of file URIs currently visible
pub fn send_visible_files_changed(hub: &Hub, uris: Vec<String>) -> Result<()> {
    let notification = ServerNotification::new(
        "visibleFilesDidChange".to_string(),
        json!({
            "uris": uris
        }),
    );

    let json = serde_json::to_string(&notification).map_err(|e| {
        AmpError::NotificationError(format!("Failed to serialize visibleFilesDidChange: {}", e))
    })?;
    hub.broadcast(&json);
    Ok(())
}

/// Send userSentMessage notification
///
/// Sends user-typed message directly to the agent.
/// This immediately submits the message to Amp CLI.
///
/// Parameters:
/// - message: The message text to send to the agent
pub fn send_user_sent_message(hub: &Hub, message: &str) -> Result<()> {
    let notification = ServerNotification::new(
        "userSentMessage".to_string(),
        json!({
            "message": message
        }),
    );

    let json = serde_json::to_string(&notification).map_err(|e| {
        AmpError::NotificationError(format!("Failed to serialize userSentMessage: {}", e))
    })?;
    hub.broadcast(&json);
    Ok(())
}

/// Send appendToPrompt notification
///
/// Appends text to the IDE prompt field without sending.
/// Allows user to edit before submitting.
///
/// Parameters:
/// - message: The text to append to the prompt field
pub fn send_append_to_prompt(hub: &Hub, message: &str) -> Result<()> {
    let notification = ServerNotification::new(
        "appendToPrompt".to_string(),
        json!({
            "message": message
        }),
    );

    let json = serde_json::to_string(&notification).map_err(|e| {
        AmpError::NotificationError(format!("Failed to serialize appendToPrompt: {}", e))
    })?;
    hub.broadcast(&json);
    Ok(())
}

/// Send diagnosticsDidChange notification
///
/// Notifies clients about updated LSP diagnostics for buffers.
///
/// Parameters:
/// - entries: Array of diagnostic entries (uri + diagnostics)
pub fn send_diagnostics_changed(hub: &Hub, entries: Vec<serde_json::Value>) -> Result<()> {
    let notification = ServerNotification::new(
        "diagnosticsDidChange".to_string(),
        json!({
            "entries": entries
        }),
    );

    let json = serde_json::to_string(&notification).map_err(|e| {
        AmpError::NotificationError(format!("Failed to serialize diagnosticsDidChange: {}", e))
    })?;
    hub.broadcast(&json);
    Ok(())
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc::unbounded_channel;

    use super::*;

    #[test]
    fn test_send_plugin_metadata() {
        let hub = Hub::new();
        let (tx, mut rx) = unbounded_channel();
        let client_id = Hub::next_client_id();

        hub.register(client_id, tx);

        let result = send_plugin_metadata(&hub, "0.1.0", "/path/to/plugin");
        assert!(result.is_ok());

        // Client should receive the notification
        let msg = rx.try_recv().unwrap();
        let value: serde_json::Value = serde_json::from_str(&msg).unwrap();

        assert!(value.get("serverNotification").is_some());
        assert_eq!(
            value["serverNotification"]["pluginMetadata"]["version"],
            "0.1.0"
        );
        assert_eq!(
            value["serverNotification"]["pluginMetadata"]["pluginDirectory"],
            "/path/to/plugin"
        );
    }

    #[test]
    fn test_send_selection_changed() {
        let hub = Hub::new();
        let (tx, mut rx) = unbounded_channel();
        let client_id = Hub::next_client_id();

        hub.register(client_id, tx);

        let result = send_selection_changed(&hub, "file:///test.txt", 10, 5, 10, 15, "selected");
        assert!(result.is_ok());

        let msg = rx.try_recv().unwrap();
        let value: serde_json::Value = serde_json::from_str(&msg).unwrap();

        assert!(value["serverNotification"]["selectionDidChange"].is_object());
        assert_eq!(
            value["serverNotification"]["selectionDidChange"]["uri"],
            "file:///test.txt"
        );

        let range = &value["serverNotification"]["selectionDidChange"]["selections"][0]["range"];
        assert_eq!(range["startLine"], 10);
        assert_eq!(range["startCharacter"], 5);
        assert_eq!(range["endLine"], 10);
        assert_eq!(range["endCharacter"], 15);
    }

    #[test]
    fn test_send_visible_files_changed() {
        let hub = Hub::new();
        let (tx, mut rx) = unbounded_channel();
        let client_id = Hub::next_client_id();

        hub.register(client_id, tx);

        let uris = vec![
            "file:///file1.txt".to_string(),
            "file:///file2.txt".to_string(),
        ];

        let result = send_visible_files_changed(&hub, uris);
        assert!(result.is_ok());

        let msg = rx.try_recv().unwrap();
        let value: serde_json::Value = serde_json::from_str(&msg).unwrap();

        assert!(value["serverNotification"]["visibleFilesDidChange"]["uris"].is_array());
        assert_eq!(
            value["serverNotification"]["visibleFilesDidChange"]["uris"][0],
            "file:///file1.txt"
        );
        assert_eq!(
            value["serverNotification"]["visibleFilesDidChange"]["uris"][1],
            "file:///file2.txt"
        );
    }

    #[test]
    fn test_broadcast_to_multiple_clients() {
        let hub = Hub::new();

        let (tx1, mut rx1) = unbounded_channel();
        let (tx2, mut rx2) = unbounded_channel();

        let id1 = Hub::next_client_id();
        let id2 = Hub::next_client_id();

        hub.register(id1, tx1);
        hub.register(id2, tx2);

        let result = send_plugin_metadata(&hub, "0.1.0", "/plugin");
        assert!(result.is_ok());

        // Both clients should receive it
        assert!(rx1.try_recv().is_ok());
        assert!(rx2.try_recv().is_ok());
    }

    #[test]
    fn test_send_user_sent_message() {
        let hub = Hub::new();
        let (tx, mut rx) = unbounded_channel();
        let client_id = Hub::next_client_id();

        hub.register(client_id, tx);

        let result = send_user_sent_message(&hub, "Hello from Neovim!");
        assert!(result.is_ok());

        let msg = rx.try_recv().unwrap();
        let value: serde_json::Value = serde_json::from_str(&msg).unwrap();

        assert!(value["serverNotification"]["userSentMessage"].is_object());
        assert_eq!(
            value["serverNotification"]["userSentMessage"]["message"],
            "Hello from Neovim!"
        );
    }

    #[test]
    fn test_send_append_to_prompt() {
        let hub = Hub::new();
        let (tx, mut rx) = unbounded_channel();
        let client_id = Hub::next_client_id();

        hub.register(client_id, tx);

        let result = send_append_to_prompt(&hub, "@file.rs#L10-L20");
        assert!(result.is_ok());

        let msg = rx.try_recv().unwrap();
        let value: serde_json::Value = serde_json::from_str(&msg).unwrap();

        assert!(value["serverNotification"]["appendToPrompt"].is_object());
        assert_eq!(
            value["serverNotification"]["appendToPrompt"]["message"],
            "@file.rs#L10-L20"
        );
    }

    #[test]
    fn test_send_user_sent_message_empty() {
        let hub = Hub::new();
        let (tx, mut rx) = unbounded_channel();
        let client_id = Hub::next_client_id();

        hub.register(client_id, tx);

        // Should handle empty messages (no validation as per amp.nvim)
        let result = send_user_sent_message(&hub, "");
        assert!(result.is_ok());

        let msg = rx.try_recv().unwrap();
        let value: serde_json::Value = serde_json::from_str(&msg).unwrap();

        assert_eq!(
            value["serverNotification"]["userSentMessage"]["message"],
            ""
        );
    }

    #[test]
    fn test_send_append_to_prompt_multiline() {
        let hub = Hub::new();
        let (tx, mut rx) = unbounded_channel();
        let client_id = Hub::next_client_id();

        hub.register(client_id, tx);

        let multiline_text = "Line 1\nLine 2\nLine 3";
        let result = send_append_to_prompt(&hub, multiline_text);
        assert!(result.is_ok());

        let msg = rx.try_recv().unwrap();
        let value: serde_json::Value = serde_json::from_str(&msg).unwrap();

        assert_eq!(
            value["serverNotification"]["appendToPrompt"]["message"],
            multiline_text
        );
    }

    #[test]
    fn test_user_messages_broadcast_to_all_clients() {
        let hub = Hub::new();

        let (tx1, mut rx1) = unbounded_channel();
        let (tx2, mut rx2) = unbounded_channel();

        let id1 = Hub::next_client_id();
        let id2 = Hub::next_client_id();

        hub.register(id1, tx1);
        hub.register(id2, tx2);

        let result = send_user_sent_message(&hub, "test message");
        assert!(result.is_ok());

        // Both clients should receive it
        let msg1 = rx1.try_recv().unwrap();
        let msg2 = rx2.try_recv().unwrap();

        let value1: serde_json::Value = serde_json::from_str(&msg1).unwrap();
        let value2: serde_json::Value = serde_json::from_str(&msg2).unwrap();

        assert_eq!(
            value1["serverNotification"]["userSentMessage"]["message"],
            "test message"
        );
        assert_eq!(
            value2["serverNotification"]["userSentMessage"]["message"],
            "test message"
        );
    }
}