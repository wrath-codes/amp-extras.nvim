///! Server-initiated notifications to Amp CLI
///!
///! This module handles sending notifications from Neovim to connected
///! Amp CLI clients, including:
///! - pluginMetadata - Plugin version and info
///! - selectionDidChange - Cursor/selection changes
///! - visibleFilesDidChange - Open files in windows

use serde_json::json;

use crate::rpc::ServerNotification;
use crate::server::Hub;

/// Send pluginMetadata notification to all clients
///
/// Sends plugin version and directory information.
/// Typically sent once when a client connects.
pub fn send_plugin_metadata(hub: &Hub, version: &str, plugin_dir: &str) {
    let notification = ServerNotification::new(
        "pluginMetadata".to_string(),
        json!({
            "version": version,
            "pluginDirectory": plugin_dir
        }),
    );
    
    let json = serde_json::to_string(&notification).unwrap();
    hub.broadcast(&json);
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
) {
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
    
    let json = serde_json::to_string(&notification).unwrap();
    hub.broadcast(&json);
}

/// Send visibleFilesDidChange notification
///
/// Notifies clients about which files are currently visible in windows.
///
/// Parameters:
/// - uris: List of file URIs currently visible
pub fn send_visible_files_changed(hub: &Hub, uris: Vec<String>) {
    let notification = ServerNotification::new(
        "visibleFilesDidChange".to_string(),
        json!({
            "uris": uris
        }),
    );
    
    let json = serde_json::to_string(&notification).unwrap();
    hub.broadcast(&json);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::unbounded;

    #[test]
    fn test_send_plugin_metadata() {
        let hub = Hub::new();
        let (tx, rx) = unbounded();
        let client_id = Hub::next_client_id();
        
        hub.register(client_id, tx);
        
        send_plugin_metadata(&hub, "0.1.0", "/path/to/plugin");
        
        // Client should receive the notification
        let msg = rx.try_recv().unwrap();
        let value: serde_json::Value = serde_json::from_str(&msg).unwrap();
        
        assert!(value.get("serverNotification").is_some());
        assert_eq!(value["serverNotification"]["pluginMetadata"]["version"], "0.1.0");
        assert_eq!(value["serverNotification"]["pluginMetadata"]["pluginDirectory"], "/path/to/plugin");
    }

    #[test]
    fn test_send_selection_changed() {
        let hub = Hub::new();
        let (tx, rx) = unbounded();
        let client_id = Hub::next_client_id();
        
        hub.register(client_id, tx);
        
        send_selection_changed(&hub, "file:///test.txt", 10, 5, 10, 15, "selected");
        
        let msg = rx.try_recv().unwrap();
        let value: serde_json::Value = serde_json::from_str(&msg).unwrap();
        
        assert!(value["serverNotification"]["selectionDidChange"].is_object());
        assert_eq!(value["serverNotification"]["selectionDidChange"]["uri"], "file:///test.txt");
        
        let range = &value["serverNotification"]["selectionDidChange"]["selections"][0]["range"];
        assert_eq!(range["startLine"], 10);
        assert_eq!(range["startCharacter"], 5);
        assert_eq!(range["endLine"], 10);
        assert_eq!(range["endCharacter"], 15);
    }

    #[test]
    fn test_send_visible_files_changed() {
        let hub = Hub::new();
        let (tx, rx) = unbounded();
        let client_id = Hub::next_client_id();
        
        hub.register(client_id, tx);
        
        let uris = vec![
            "file:///file1.txt".to_string(),
            "file:///file2.txt".to_string(),
        ];
        
        send_visible_files_changed(&hub, uris);
        
        let msg = rx.try_recv().unwrap();
        let value: serde_json::Value = serde_json::from_str(&msg).unwrap();
        
        assert!(value["serverNotification"]["visibleFilesDidChange"]["uris"].is_array());
        assert_eq!(value["serverNotification"]["visibleFilesDidChange"]["uris"][0], "file:///file1.txt");
        assert_eq!(value["serverNotification"]["visibleFilesDidChange"]["uris"][1], "file:///file2.txt");
    }

    #[test]
    fn test_broadcast_to_multiple_clients() {
        let hub = Hub::new();
        
        let (tx1, rx1) = unbounded();
        let (tx2, rx2) = unbounded();
        
        let id1 = Hub::next_client_id();
        let id2 = Hub::next_client_id();
        
        hub.register(id1, tx1);
        hub.register(id2, tx2);
        
        send_plugin_metadata(&hub, "0.1.0", "/plugin");
        
        // Both clients should receive it
        assert!(rx1.try_recv().is_ok());
        assert!(rx2.try_recv().is_ok());
    }
}
