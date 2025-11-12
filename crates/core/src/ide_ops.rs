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
use std::path::{Path, PathBuf};

use crate::errors::{AmpError, Result};

// ============================================================================
// Neovim Context Detection
// ============================================================================

/// Flag indicating Neovim is initialized and ready for API calls
static NVIM_READY: OnceCell<()> = OnceCell::new();

/// Mark Neovim as ready for API calls
///
/// Should be called during plugin initialization (after AsyncHandle setup)
pub fn mark_nvim_ready() {
    let _ = NVIM_READY.set(());
}

/// Check if Neovim API is available
///
/// Returns true if we're running inside Neovim and API calls are safe
fn nvim_available() -> bool {
    NVIM_READY.get().is_some()
}

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

/// Work to schedule on Neovim main thread
type ScheduledWork = Box<dyn FnOnce() + Send>;

/// Global channel for scheduling work on Neovim main thread
static WORK_TX: OnceCell<Sender<ScheduledWork>> = OnceCell::new();

/// Global AsyncHandle for scheduled work
static WORK_HANDLE: OnceCell<AsyncHandle> = OnceCell::new();

/// Initialize the AsyncHandle and message channel for IDE operations
///
/// Must be called before starting the server to enable nvim/notify.
pub fn init_async_handle() -> Result<()> {
    // Initialize notification channel
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
    
    // Initialize work scheduler channel
    let (work_tx, work_rx): (Sender<ScheduledWork>, Receiver<ScheduledWork>) = unbounded();
    
    // Create AsyncHandle for scheduled work
    let work_handle = AsyncHandle::new(move || {
        // Process all pending work
        while let Ok(work) = work_rx.try_recv() {
            work();
        }
        Ok::<_, std::convert::Infallible>(())
    })
    .map_err(|e| AmpError::Other(format!("Failed to create work AsyncHandle: {}", e)))?;
    
    let _ = WORK_TX.set(work_tx);
    let _ = WORK_HANDLE.set(work_handle);
    
    // Mark Neovim as ready for API calls
    mark_nvim_ready();
    
    Ok(())
}

/// Schedule work to run on Neovim's main thread
///
/// This is used to safely call Neovim APIs from background threads.
pub fn schedule_on_main_thread<F>(work: F) -> Result<()>
where
    F: FnOnce() + Send + 'static,
{
    if let Some(tx) = WORK_TX.get() {
        if let Some(handle) = WORK_HANDLE.get() {
            tx.send(Box::new(work))
                .map_err(|_| AmpError::Other("Failed to send work to scheduler".into()))?;
            handle.send()
                .map_err(|e| AmpError::Other(format!("Failed to trigger work handle: {}", e)))?;
            Ok(())
        } else {
            Err(AmpError::Other("Work handle not initialized".into()))
        }
    } else {
        Err(AmpError::Other("Work scheduler not initialized".into()))
    }
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

/// Parameters for getDiagnostics
#[derive(Debug, Deserialize)]
struct GetDiagnosticsParams {
    path: Option<String>,
}

/// Diagnostic entry from Neovim's vim.diagnostic.get()
///
/// Fields are 0-based as returned by Neovim
#[derive(Debug, Deserialize)]
struct NvimDiagnostic {
    lnum: u32,
    col: u32,
    end_lnum: Option<u32>,
    end_col: Option<u32>,
    severity: Option<u8>,
    message: String,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Find a Neovim buffer by its file path
///
/// Searches all buffers for one matching the given absolute path.
/// Prefers loaded buffers over unloaded ones.
///
/// # Arguments
/// * `path` - Absolute file path to search for
///
/// # Returns
/// * `Some(Buffer)` if found (preferring loaded buffers)
/// * `None` if no buffer matches the path
#[cfg(not(test))]
fn find_buffer_by_path(path: &Path) -> Option<nvim_oxi::api::Buffer> {
    use nvim_oxi::api;
    
    let mut fallback: Option<api::Buffer> = None;
    
    for buf in api::list_bufs() {
        // Get buffer name (file path)
        let Ok(buf_path) = buf.get_name() else {
            continue;
        };
        
        // Only consider absolute paths (skip unnamed/scratch buffers)
        if !buf_path.is_absolute() {
            continue;
        }
        
        // Check if path matches
        if buf_path == path {
            // Prefer loaded buffers
            if buf.is_loaded() {
                return Some(buf);
            }
            // Remember as fallback if not loaded
            fallback = Some(buf);
        }
    }
    
    fallback
}

/// Normalize a path to absolute form
///
/// Uses Neovim's fnamemodify to expand relative paths.
/// Falls back to returning the path as-is if Neovim is not available.
///
/// # Arguments
/// * `path` - Path to normalize (can be relative or absolute)
///
/// # Returns
/// * Normalized absolute path
fn normalize_path(path: &str) -> Result<PathBuf> {
    let path_buf = PathBuf::from(path);
    
    // If already absolute, return as-is
    if path_buf.is_absolute() {
        return Ok(path_buf);
    }
    
    // Try to normalize using Neovim if available
    #[cfg(not(test))]
    if nvim_available() {
        use nvim_oxi::api;
        
        // Use vim.fn.fnamemodify(path, ':p') to get absolute path
        let lua_expr = "vim.fn.fnamemodify(args, ':p')";
        let result: std::result::Result<nvim_oxi::Object, _> = api::call_function(
            "luaeval",
            (lua_expr, path),
        );
        
        if let Ok(obj) = result {
            use nvim_oxi::conversion::FromObject;
            if let Ok(normalized) = <String as FromObject>::from_object(obj) {
                return Ok(PathBuf::from(normalized));
            }
        }
    }
    
    // Fallback: use current directory
    let cwd = std::env::current_dir()
        .map_err(|e| AmpError::Other(format!("Failed to get current directory: {}", e)))?;
    Ok(cwd.join(path))
}

/// Map Neovim diagnostic severity to amp.nvim string format
///
/// Neovim severity levels:
/// - 1 = ERROR
/// - 2 = WARN
/// - 3 = INFO
/// - 4 = HINT
#[cfg(not(test))]
fn map_severity(severity: Option<u8>) -> &'static str {
    match severity.unwrap_or(3) {
        1 => "error",
        2 => "warning",
        3 => "info",
        4 => "hint",
        _ => "info", // Default to info for unknown values
    }
}

/// Get line content for a diagnostic
///
/// Tries to read from buffer if loaded, falls back to disk
#[cfg(not(test))]
fn get_line_content(path: &Path, line_num: u32) -> String {
    // Try buffer first
    if let Some(buf) = find_buffer_by_path(path) {
        if buf.is_loaded() {
            if let Ok(lines) = buf.get_lines(line_num as usize..(line_num as usize + 1), false) {
                if let Some(line) = lines.into_iter().next() {
                    return line.to_string_lossy().into_owned();
                }
            }
        }
    }
    
    // Fallback to disk
    if let Ok(content) = fs::read_to_string(path) {
        if let Some(line) = content.lines().nth(line_num as usize) {
            return line.to_string();
        }
    }
    
    String::new()
}

/// Implementation of getDiagnostics (only compiled when not in test mode)
#[cfg(not(test))]
fn get_diagnostics_impl(path_filter: Option<&str>) -> Result<Value> {
    use nvim_oxi::api;
    use nvim_oxi::conversion::FromObject;
    use std::collections::HashMap;
    
    // Collect diagnostics grouped by file URI
    let mut entries_map: HashMap<String, Vec<Value>> = HashMap::new();
    
    // Iterate through all buffers
    for buf in api::list_bufs() {
        // Get buffer path
        let Ok(buf_path) = buf.get_name() else {
            continue;
        };
        
        // Only consider absolute paths
        if !buf_path.is_absolute() {
            continue;
        }
        
        let path_str = buf_path.to_string_lossy().to_string();
        
        // Apply path filter (prefix matching for directories)
        if let Some(filter) = path_filter {
            if !path_str.starts_with(filter) {
                continue;
            }
        }
        
        // Get diagnostics for this buffer using luaeval
        let lua_expr = "vim.json.encode(vim.diagnostic.get(vim.fn.bufnr(args)))";
        let result: std::result::Result<nvim_oxi::Object, _> = api::call_function(
            "luaeval",
            (lua_expr, path_str.clone()),
        );
        
        let Ok(diag_json_obj) = result else {
            continue;
        };
        
        // Convert Object to String using FromObject
        let diag_json_str = match <String as FromObject>::from_object(diag_json_obj) {
            Ok(s) => s,
            Err(_) => continue,
        };
        
        // Parse JSON into Vec<NvimDiagnostic>
        let diags: Vec<NvimDiagnostic> = match serde_json::from_str(&diag_json_str) {
            Ok(d) => d,
            Err(_) => continue,
        };
        
        // Skip if no diagnostics for this buffer
        if diags.is_empty() {
            continue;
        }
        
        // Convert to amp.nvim format
        let uri = format!("file://{}", path_str);
        let diagnostics: Vec<Value> = diags
            .into_iter()
            .map(|diag| {
                let line_content = get_line_content(&buf_path, diag.lnum);
                let start_line = diag.lnum;
                let start_char = diag.col;
                let end_line = diag.end_lnum.unwrap_or(diag.lnum);
                let end_char = diag.end_col.unwrap_or(diag.col);
                
                // Calculate character offsets (simple approach)
                let start_offset = start_char;
                let end_offset = end_char;
                
                json!({
                    "range": {
                        "startLine": start_line,
                        "startCharacter": start_char,
                        "endLine": end_line,
                        "endCharacter": end_char
                    },
                    "severity": map_severity(diag.severity),
                    "description": diag.message,
                    "lineContent": line_content,
                    "startOffset": start_offset,
                    "endOffset": end_offset
                })
            })
            .collect();
        
        entries_map.insert(uri, diagnostics);
    }
    
    // Convert to entries array
    let entries: Vec<Value> = entries_map
        .into_iter()
        .map(|(uri, diagnostics)| {
            json!({
                "uri": uri,
                "diagnostics": diagnostics
            })
        })
        .collect();
    
    Ok(json!({
        "entries": entries
    }))
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
/// Integrates with Neovim's diagnostic system.
///
/// Request:
/// ```json
/// { "path": "/path/to/file.txt" }  // Optional - file or directory prefix
/// ```
///
/// Response:
/// ```json
/// {
///   "entries": [
///     {
///       "uri": "file:///path/to/file.rs",
///       "diagnostics": [...]
///     }
///   ]
/// }
/// ```
pub fn get_diagnostics(params: Value) -> Result<Value> {
    let params: GetDiagnosticsParams = serde_json::from_value(params)
        .map_err(|e| AmpError::InvalidArgs {
            command: "getDiagnostics".to_string(),
            reason: e.to_string(),
        })?;
    
    // Only get diagnostics if Neovim is initialized
    #[cfg(not(test))]
    if nvim_available() {
        return get_diagnostics_impl(params.path.as_deref());
    }
    
    // Fallback: return empty diagnostics
    Ok(json!({
        "entries": []
    }))
}

/// Handle ide/readFile request
///
/// Reads file content, preferring Neovim buffer content over disk.
/// This ensures unsaved changes are captured.
///
/// Request:
/// ```json
/// { "path": "/absolute/path/to/file.txt" }
/// ```
///
/// Response:
/// ```json
/// {
///   "success": true,
///   "content": "file content here...",
///   "encoding": "utf-8"
/// }
/// ```
pub fn read_file(params: Value) -> Result<Value> {
    let params: ReadFileParams = serde_json::from_value(params)
        .map_err(|e| AmpError::InvalidArgs {
            command: "ide/readFile".to_string(),
            reason: e.to_string(),
        })?;

    // Normalize path (handles both absolute and relative paths)
    let path = normalize_path(&params.path)?;

    // Try to read from Neovim buffer first (to get unsaved changes)
    // Only if Neovim is initialized and ready
    #[cfg(not(test))]
    if nvim_available() {
        if let Some(buf) = find_buffer_by_path(&path) {
            if buf.is_loaded() {
                // Get line count and read all lines
                if let Ok(line_count) = buf.line_count() {
                    let lines_result: std::result::Result<Vec<String>, _> = buf
                        .get_lines(0..line_count, false)
                        .map(|iter| {
                            iter.map(|s| s.to_string_lossy().into_owned())
                                .collect()
                        });
                    
                    if let Ok(lines) = lines_result {
                        let content = lines.join("\n");
                        return Ok(json!({
                            "success": true,
                            "content": content,
                            "encoding": "utf-8"
                        }));
                    }
                }
            }
        }
    }

    // Fall back to reading from disk
    let content = fs::read_to_string(&path).map_err(|e| {
        AmpError::IoError(std::io::Error::new(
            e.kind(),
            format!("Failed to read file {}: {}", params.path, e),
        ))
    })?;

    Ok(json!({
        "success": true,
        "content": content,
        "encoding": "utf-8"
    }))
}

/// Handle ide/editFile request
///
/// Writes file content, updating Neovim buffer if it exists.
/// This ensures edits appear immediately in the editor.
///
/// Request:
/// ```json
/// { "path": "/absolute/path/to/file.txt", "content": "new content" }
/// ```
///
/// Response:
/// ```json
/// {
///   "success": true,
///   "message": "Wrote 123 bytes to /path/to/file.txt",
///   "appliedChanges": true
/// }
/// ```
pub fn edit_file(params: Value) -> Result<Value> {
    let params: EditFileParams = serde_json::from_value(params)
        .map_err(|e| AmpError::InvalidArgs {
            command: "ide/editFile".to_string(),
            reason: e.to_string(),
        })?;

    // Normalize path (handles both absolute and relative paths)
    let path = normalize_path(&params.path)?;

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

    // Prepare lines from content (ensure at least one line for Neovim)
    let mut lines: Vec<String> = params.content
        .split('\n')
        .map(|s| s.to_string())
        .collect();
    
    if lines.is_empty() {
        lines.push(String::new());
    }

    // Update Neovim buffer if it exists (only if Neovim is initialized and ready)
    // Note: We only update existing buffers, not create new ones.
    // Creating buffers can cause swap file conflicts (E325) when users open files later.
    #[cfg(not(test))]
    if nvim_available() {
        if let Some(mut buf) = find_buffer_by_path(&path) {
            // Replace entire buffer content
            if let Ok(line_count) = buf.line_count() {
                buf.set_lines(0..line_count, false, lines.clone())
                    .map_err(|e| AmpError::Other(format!("Failed to set buffer lines: {}", e)))?;
            }
            
            // Mark buffer as unmodified (we're about to save to disk)
            buf.set_option("modified", false)
                .map_err(|e| AmpError::Other(format!("Failed to set 'modified' option: {}", e)))?;
        }
        // If no buffer exists, just write to disk and let Neovim handle buffer creation
        // when the user opens the file naturally
    }

    // Write to disk
    fs::write(&path, &params.content).map_err(|e| {
        AmpError::IoError(std::io::Error::new(
            e.kind(),
            format!("Failed to write file {}: {}", params.path, e),
        ))
    })?;

    Ok(json!({
        "success": true,
        "message": format!("Wrote {} bytes to {}", params.content.len(), params.path),
        "appliedChanges": true
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

        assert_eq!(result["success"], json!(true));
        assert_eq!(result["content"], json!("test content"));
        assert_eq!(result["encoding"], json!("utf-8"));
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
        use tempfile::tempdir;
        
        // Relative paths are now normalized to absolute
        // Create a temp directory with a test file
        let temp_dir = tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();
        
        // Create the relative file
        fs::create_dir_all(temp_dir.path().join("relative")).unwrap();
        fs::write(temp_dir.path().join("relative").join("path.txt"), "relative content").unwrap();
        
        let result = read_file(json!({ "path": "relative/path.txt" }));

        // Should succeed - path gets normalized and file is found
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response["success"], json!(true));
        assert_eq!(response["content"], json!("relative content"));
        assert_eq!(response["encoding"], json!("utf-8"));
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
        let response = result.unwrap();
        assert_eq!(response["success"], json!(true));
        assert_eq!(response["appliedChanges"], json!(true));
        assert!(response["message"].is_string());

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
        use tempfile::tempdir;
        
        // Relative paths are now normalized to absolute
        // Create a temp directory to use as current_dir
        let temp_dir = tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();
        
        let result = edit_file(json!({
            "path": "relative/path.txt",
            "content": "content"
        }));

        // Should succeed - path gets normalized
        assert!(result.is_ok());
        
        // Verify file was created at normalized path
        let normalized_path = temp_dir.path().join("relative").join("path.txt");
        assert!(normalized_path.exists());
        let content = fs::read_to_string(&normalized_path).unwrap();
        assert_eq!(content, "content");
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
