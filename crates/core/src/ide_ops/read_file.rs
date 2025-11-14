//! File reading operation

use std::fs;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::errors::{AmpError, Result};

/// Parameters for ide/readFile
#[derive(Debug, Deserialize)]
struct ReadFileParams {
    path: String,
}

/// Handle ide/readFile request
///
/// Reads file content, checking Neovim buffers first for unsaved changes,
/// then falling back to disk.
///
/// Request:
/// ```json
/// { "path": "/path/to/file.txt" }
/// ```
///
/// Response:
/// ```json
/// {
///   "success": true,
///   "content": "file content here",
///   "encoding": "utf-8"
/// }
/// ```
///
/// # Errors
/// - InvalidArgs: Missing or invalid path parameter
/// - IoError: File not found or read error
pub fn read_file(params: Value) -> Result<Value> {
    let params: ReadFileParams =
        serde_json::from_value(params).map_err(|e| AmpError::InvalidArgs {
            command: "ide/readFile".to_string(),
            reason:  e.to_string(),
        })?;

    // Normalize path (handles both absolute and relative paths)
    let path = super::normalize_path(&params.path)?;

    // Try to read from Neovim buffer first (to get unsaved changes)
    // Only if Neovim is initialized and ready
    #[cfg(not(test))]
    if super::nvim_available() {
        if let Some(buf) = super::find_buffer_by_path(&path) {
            if buf.is_loaded() {
                // Get line count and read all lines
                if let Ok(line_count) = buf.line_count() {
                    let lines_result: std::result::Result<Vec<String>, _> = buf
                        .get_lines(0..line_count, false)
                        .map(|iter| {
                            iter.map(|s| {
                                s.to_str()
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|_| String::new())
                            })
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

#[cfg(test)]
mod tests {
    use tempfile::NamedTempFile;

    use super::*;

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
            },
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
        fs::write(
            temp_dir.path().join("relative").join("path.txt"),
            "relative content",
        )
        .unwrap();

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
            },
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
            },
            _ => panic!("Expected InvalidArgs"),
        }
    }
}
