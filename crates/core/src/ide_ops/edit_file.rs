//! File writing operation

use std::fs;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::errors::{AmpError, Result};

/// Parameters for ide/editFile
#[derive(Debug, Deserialize)]
struct EditFileParams {
    path:    String,
    content: String,
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
///
/// # Errors
/// - InvalidArgs: Missing or invalid parameters
/// - IoError: Failed to create directory or write file
pub fn edit_file(params: Value) -> Result<Value> {
    let params: EditFileParams =
        serde_json::from_value(params).map_err(|e| AmpError::InvalidArgs {
            command: "ide/editFile".to_string(),
            reason:  e.to_string(),
        })?;

    // Normalize path (handles both absolute and relative paths)
    let path = super::normalize_path(&params.path)?;

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
    let mut lines: Vec<String> = params.content.split('\n').map(|s| s.to_string()).collect();

    if lines.is_empty() {
        lines.push(String::new());
    }

    // Update Neovim buffer if it exists (only if Neovim is initialized and ready)
    // Note: We only update existing buffers, not create new ones.
    // Creating buffers can cause swap file conflicts (E325) when users open files
    // later.
    #[cfg(not(test))]
    if super::nvim_available() {
        if let Some(mut buf) = super::find_buffer_by_path(&path) {
            // Replace entire buffer content
            if let Ok(line_count) = buf.line_count() {
                buf.set_lines(0..line_count, false, lines.clone())
                    .map_err(|e| AmpError::Other(format!("Failed to set buffer lines: {}", e)))?;
            }

            // Mark buffer as unmodified (we're about to save to disk)
            buf.set_option("modified", false)
                .map_err(|e| AmpError::Other(format!("Failed to set 'modified' option: {}", e)))?;
        }
        // If no buffer exists, just write to disk and let Neovim handle buffer
        // creation when the user opens the file naturally
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
        "message": nvim_oxi::string!("Wrote {} bytes to {}", params.content.len(), params.path).to_string(),
        "appliedChanges": true
    }))
}

#[cfg(test)]
mod tests {
    use tempfile::{tempdir, NamedTempFile};

    use super::*;

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
            },
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
}
