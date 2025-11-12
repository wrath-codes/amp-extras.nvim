//! IDE protocol operations
//!
//! Implements IDE-specific methods that Amp CLI can call:
//! - ide/ping - Health check
//! - ide/readFile - Read file content
//! - ide/editFile - Write file content
//! - nvim/notify - Send notification to Neovim

use crossbeam_channel::{unbounded, Receiver, Sender};
use nvim_oxi::libuv::AsyncHandle;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

use crate::errors::{AmpError, Result};

// ============================================================================
// Global AsyncHandle and Notification Channel
// ============================================================================

/// Message to send to Neovim
#[derive(Debug, Clone)]
struct NvimMessage {
    message: String,
}

/// Global channel for sending messages to Neovim
static NVIM_TX: OnceCell<Sender<NvimMessage>> = OnceCell::new();

/// Global AsyncHandle for triggering Neovim callbacks
static ASYNC_HANDLE: OnceCell<AsyncHandle> = OnceCell::new();

/// Initialize the AsyncHandle and message channel for IDE operations
///
/// Must be called before starting the server to enable nvim/notify.
pub fn init_async_handle() -> Result<()> {
    let (tx, rx): (Sender<NvimMessage>, Receiver<NvimMessage>) = unbounded();
    
    // Create AsyncHandle with callback that processes messages from channel
    let handle = AsyncHandle::new(move || {
        // Process all pending messages
        while let Ok(msg) = rx.try_recv() {
            let lua_code = format!(
                "vim.notify([[{}]], vim.log.levels.INFO)",
                msg.message.replace("\\", "\\\\").replace("[[", "").replace("]]", "")
            );
            let _ = nvim_oxi::api::exec(&lua_code, false);
        }
        Ok::<_, std::convert::Infallible>(())
    })
    .map_err(|e| AmpError::Other(format!("Failed to create AsyncHandle: {}", e)))?;
    
    let _ = NVIM_TX.set(tx);
    let _ = ASYNC_HANDLE.set(handle);
    
    Ok(())
}

// ============================================================================
// Request/Response Types
// ============================================================================

/// Parameters for ide/readFile
#[derive(Debug, Deserialize)]
struct ReadFileParams {
    path: String,
}

/// Parameters for ide/editFile
#[derive(Debug, Deserialize)]
struct EditFileParams {
    path: String,
    content: String,
}

/// Parameters for nvim/notify
#[derive(Debug, Deserialize)]
struct NotifyParams {
    message: String,
}

// ============================================================================
// IDE Operations
// ============================================================================

/// Handle ping request (both ide/ping and ping)
///
/// Returns:
/// ```json
/// { "pong": true, "ts": "2025-01-11T12:00:00Z" }
/// ```
///
/// For amp.nvim protocol, can also accept and echo message:
/// ```json
/// { "message": "hello" }
/// ```
pub fn ping(params: Value) -> Result<Value> {
    // Check if there's a message to echo (amp.nvim format)
    if let Some(message) = params.get("message") {
        Ok(json!({
            "message": message
        }))
    } else {
        // Standard ping response
        Ok(json!({
            "pong": true,
            "ts": chrono::Utc::now().to_rfc3339()
        }))
    }
}

/// Handle authenticate request
///
/// Simple authentication handshake for IDE protocol.
/// In the amp.nvim implementation, this just acknowledges the connection.
///
/// Returns:
/// ```json
/// { "authenticated": true }
/// ```
pub fn authenticate(_params: Value) -> Result<Value> {
    Ok(json!({
        "authenticated": true
    }))
}

/// Handle getDiagnostics request
///
/// Returns diagnostics for a file or directory path.
/// Currently returns empty diagnostics (stub implementation).
///
/// Request:
/// ```json
/// { "path": "/path/to/file.txt" }
/// ```
///
/// Response:
/// ```json
/// { "entries": [] }
/// ```
///
/// TODO: Integrate with Neovim's diagnostic system via nvim-oxi
pub fn get_diagnostics(_params: Value) -> Result<Value> {
    // For now, return empty diagnostics
    // In the future, this should:
    // 1. Parse the path parameter
    // 2. Get diagnostics from Neovim's diagnostic API
    // 3. Format them according to amp.nvim protocol
    
    Ok(json!({
        "entries": []
    }))
}

/// Handle ide/readFile request
///
/// Reads file content from filesystem.
///
/// Request:
/// ```json
/// { "path": "/absolute/path/to/file.txt" }
/// ```
///
/// Response:
/// ```json
/// { "content": "file content here..." }
/// ```
pub fn read_file(params: Value) -> Result<Value> {
    let params: ReadFileParams = serde_json::from_value(params)
        .map_err(|e| AmpError::InvalidArgs {
            command: "ide/readFile".to_string(),
            reason: e.to_string(),
        })?;

    // Validate path
    let path = Path::new(&params.path);
    if !path.is_absolute() {
        return Err(AmpError::ValidationError(
            "Path must be absolute".to_string(),
        ));
    }

    // Read file
    let content = fs::read_to_string(path).map_err(|e| {
        AmpError::IoError(std::io::Error::new(
            e.kind(),
            format!("Failed to read file {}: {}", params.path, e),
        ))
    })?;

    Ok(json!({
        "content": content
    }))
}

/// Handle ide/editFile request
///
/// Writes file content to filesystem (whole-file replacement).
///
/// Request:
/// ```json
/// { "path": "/absolute/path/to/file.txt", "content": "new content" }
/// ```
///
/// Response:
/// ```json
/// { "success": true }
/// ```
pub fn edit_file(params: Value) -> Result<Value> {
    let params: EditFileParams = serde_json::from_value(params)
        .map_err(|e| AmpError::InvalidArgs {
            command: "ide/editFile".to_string(),
            reason: e.to_string(),
        })?;

    // Validate path
    let path = Path::new(&params.path);
    if !path.is_absolute() {
        return Err(AmpError::ValidationError(
            "Path must be absolute".to_string(),
        ));
    }

    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| {
                AmpError::IoError(std::io::Error::new(
                    e.kind(),
                    format!("Failed to create parent directory: {}", e),
                ))
            })?;
        }
    }

    // Write file
    fs::write(path, params.content).map_err(|e| {
        AmpError::IoError(std::io::Error::new(
            e.kind(),
            format!("Failed to write file {}: {}", params.path, e),
        ))
    })?;

    Ok(json!({
        "success": true
    }))
}

/// Handle nvim/notify notification
///
/// Sends a notification to Neovim via AsyncHandle.
///
/// Request:
/// ```json
/// { "message": "Hello from Amp CLI", "level": "info" }
/// ```
pub fn nvim_notify(params: Value) -> Result<()> {
    let params: NotifyParams = serde_json::from_value(params)
        .map_err(|e| AmpError::InvalidArgs {
            command: "nvim/notify".to_string(),
            reason: e.to_string(),
        })?;

    // Get channel sender
    let tx = NVIM_TX.get()
        .ok_or_else(|| AmpError::Other("AsyncHandle not initialized".into()))?;

    // Get AsyncHandle to trigger callback
    let async_handle = ASYNC_HANDLE.get()
        .ok_or_else(|| AmpError::Other("AsyncHandle not initialized".into()))?;

    // Send message through channel
    let msg = NvimMessage {
        message: params.message,
    };
    
    tx.send(msg)
        .map_err(|e| AmpError::Other(format!("Failed to send message: {}", e)))?;
    
    // Trigger AsyncHandle to process the message
    async_handle.send()
        .map_err(|e| AmpError::Other(format!("Failed to trigger AsyncHandle: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{tempdir, NamedTempFile};

    // ========================================
    // ide/ping tests
    // ========================================

    #[test]
    fn test_ping_returns_pong() {
        let result = ping(json!({})).unwrap();

        assert_eq!(result["pong"], json!(true));
        assert!(result.get("ts").is_some());
    }

    #[test]
    fn test_ping_with_message() {
        // amp.nvim format - echo message
        let result = ping(json!({"message": "hello"})).unwrap();

        assert_eq!(result["message"], json!("hello"));
    }

    #[test]
    fn test_ping_without_message() {
        // Standard format - return pong
        let result = ping(json!({"other": "data"})).unwrap();

        assert_eq!(result["pong"], json!(true));
        assert!(result.get("ts").is_some());
    }

    #[test]
    fn test_ping_timestamp_format() {
        let result = ping(json!({})).unwrap();
        let ts = result["ts"].as_str().unwrap();

        // Should be valid RFC3339 timestamp
        assert!(chrono::DateTime::parse_from_rfc3339(ts).is_ok());
    }

    // ========================================
    // authenticate tests
    // ========================================

    #[test]
    fn test_authenticate_success() {
        let result = authenticate(json!({})).unwrap();

        assert_eq!(result["authenticated"], json!(true));
    }

    #[test]
    fn test_authenticate_with_params() {
        // authenticate ignores params for now
        let result = authenticate(json!({"token": "abc123"})).unwrap();

        assert_eq!(result["authenticated"], json!(true));
    }

    // ========================================
    // getDiagnostics tests
    // ========================================

    #[test]
    fn test_get_diagnostics_empty() {
        let result = get_diagnostics(json!({"path": "/tmp/test.txt"})).unwrap();

        assert!(result["entries"].is_array());
        assert_eq!(result["entries"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_get_diagnostics_no_params() {
        // Should still return empty for now
        let result = get_diagnostics(json!({})).unwrap();

        assert!(result["entries"].is_array());
    }

    // ========================================
    // ide/readFile tests
    // ========================================

    #[test]
    fn test_read_file_success() {
        // Create temp file with content
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();
        fs::write(path, "test content").unwrap();

        let result = read_file(json!({ "path": path })).unwrap();

        assert_eq!(result["content"], json!("test content"));
    }

    #[test]
    fn test_read_file_nonexistent() {
        let result = read_file(json!({ "path": "/tmp/nonexistent_file_12345.txt" }));

        assert!(result.is_err());
        match result {
            Err(AmpError::IoError(_)) => {
                // Expected
            }
            _ => panic!("Expected IoError"),
        }
    }

    #[test]
    fn test_read_file_relative_path() {
        let result = read_file(json!({ "path": "relative/path.txt" }));

        assert!(result.is_err());
        match result {
            Err(AmpError::ValidationError(msg)) => {
                assert!(msg.contains("absolute"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[test]
    fn test_read_file_missing_path_param() {
        let result = read_file(json!({}));

        assert!(result.is_err());
        match result {
            Err(AmpError::InvalidArgs { command, .. }) => {
                assert_eq!(command, "ide/readFile");
            }
            _ => panic!("Expected InvalidArgs"),
        }
    }

    #[test]
    fn test_read_file_invalid_params() {
        let result = read_file(json!({ "path": 123 }));

        assert!(result.is_err());
        match result {
            Err(AmpError::InvalidArgs { .. }) => {
                // Expected
            }
            _ => panic!("Expected InvalidArgs"),
        }
    }

    // ========================================
    // ide/editFile tests
    // ========================================

    #[test]
    fn test_edit_file_success() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let path_str = file_path.to_str().unwrap();

        let result = edit_file(json!({
            "path": path_str,
            "content": "new content"
        }));

        assert!(result.is_ok());
        assert_eq!(result.unwrap()["success"], json!(true));

        // Verify file was written
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "new content");
    }

    #[test]
    fn test_edit_file_creates_parent_dir() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("nested").join("dir").join("test.txt");
        let path_str = file_path.to_str().unwrap();

        let result = edit_file(json!({
            "path": path_str,
            "content": "content"
        }));

        assert!(result.is_ok());

        // Verify file and directories were created
        assert!(file_path.exists());
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "content");
    }

    #[test]
    fn test_edit_file_overwrites_existing() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();
        fs::write(path, "old content").unwrap();

        let result = edit_file(json!({
            "path": path,
            "content": "new content"
        }));

        assert!(result.is_ok());

        // Verify file was overwritten
        let content = fs::read_to_string(path).unwrap();
        assert_eq!(content, "new content");
    }

    #[test]
    fn test_edit_file_relative_path() {
        let result = edit_file(json!({
            "path": "relative/path.txt",
            "content": "content"
        }));

        assert!(result.is_err());
        match result {
            Err(AmpError::ValidationError(msg)) => {
                assert!(msg.contains("absolute"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[test]
    fn test_edit_file_missing_params() {
        let result = edit_file(json!({ "path": "/tmp/test.txt" }));

        assert!(result.is_err());
        match result {
            Err(AmpError::InvalidArgs { command, .. }) => {
                assert_eq!(command, "ide/editFile");
            }
            _ => panic!("Expected InvalidArgs"),
        }
    }

    #[test]
    fn test_edit_file_empty_content() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        let result = edit_file(json!({
            "path": path,
            "content": ""
        }));

        assert!(result.is_ok());

        // Verify empty file was written
        let content = fs::read_to_string(path).unwrap();
        assert_eq!(content, "");
    }

    // ========================================
    // nvim/notify tests
    // ========================================

    #[test]
    fn test_nvim_notify_without_async_handle() {
        // Without AsyncHandle, should return error
        let result = nvim_notify(json!({
            "message": "Test notification"
        }));

        assert!(result.is_err());
        match result {
            Err(AmpError::Other(msg)) => {
                assert!(msg.contains("AsyncHandle not initialized"));
            }
            _ => panic!("Expected 'AsyncHandle not initialized' error"),
        }
    }

    #[test]
    fn test_nvim_notify_missing_message() {
        let result = nvim_notify(json!({}));

        assert!(result.is_err());
        match result {
            Err(AmpError::InvalidArgs { command, .. }) => {
                assert_eq!(command, "nvim/notify");
            }
            _ => panic!("Expected InvalidArgs"),
        }
    }

    #[test]
    fn test_nvim_notify_invalid_params() {
        let result = nvim_notify(json!({ "message": 123 }));

        assert!(result.is_err());
        match result {
            Err(AmpError::InvalidArgs { .. }) => {
                // Expected
            }
            _ => panic!("Expected InvalidArgs"),
        }
    }
}
