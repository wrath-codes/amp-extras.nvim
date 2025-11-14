//! Send file reference with line number to Amp prompt
//!
//! Uses nvim-oxi to get current buffer path and cursor position,
//! creates a reference in the format `@file.rs#L10`.

use nvim_oxi::api::{Buffer, Window};
use serde::Serialize;
use serde_json::{json, Value};

use crate::{
    errors::{AmpError, Result},
    notifications,
};

/// Response for send_line_ref command
#[derive(Debug, Serialize)]
pub struct SendLineRefResponse {
    /// Success flag
    pub success:   bool,
    /// The formatted reference that was sent
    pub reference: String,
}

/// Send file reference with line number to Amp prompt
///
/// Uses nvim-oxi to get the current buffer path and cursor line,
/// formats a reference like `@file.rs#L10`, and sends it to the Amp prompt.
///
/// # Request
/// ```json
/// {}
/// ```
///
/// # Response
/// ```json
/// {
///   "success": true,
///   "reference": "@src/main.rs#L10"
/// }
/// ```
///
/// # Errors
/// - Returns error if buffer has no filename
/// - Returns error if WebSocket server is not running
/// - Returns error if notification fails to send
pub fn send_line_ref(_params: Value) -> Result<Value> {
    // Get current buffer
    let buf = Buffer::current();

    // Get buffer name
    let bufname = buf
        .get_name()
        .map_err(|e| AmpError::Other(format!("Failed to get buffer name: {}", e)))?;

    if bufname.to_string_lossy().is_empty() {
        return Err(AmpError::Other("Current buffer has no filename".into()));
    }

    // Get relative path using nvim utilities
    let file_path = crate::nvim::path::to_relative(&bufname)?;

    // Get current cursor position
    let win = Window::current();
    let (line, _col) = win
        .get_cursor()
        .map_err(|e| AmpError::Other(format!("Failed to get cursor position: {}", e)))?;

    // Get Hub from server
    let hub = crate::server::get_hub()
        .ok_or_else(|| AmpError::Other("WebSocket server not running".into()))?;

    // Format reference: @file.rs#L10 (line is 1-indexed from nvim-oxi)
    let reference = format!("@{}#L{}", file_path, line);

    // Send reference to prompt
    notifications::send_append_to_prompt(&hub, &reference)?;

    Ok(json!(SendLineRefResponse {
        success: true,
        reference
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_line_ref_accepts_empty_params() {
        // send_line_ref doesn't need params
        let _params = json!({});
        // Just verify it doesn't panic on deserialization
        // Actual execution requires Neovim context
    }

    #[test]
    #[ignore = "Requires Neovim context"]
    fn test_send_line_ref_without_server() {
        // Ensure server is stopped
        crate::server::stop();

        let params = json!({});
        let result = send_line_ref(params);
        assert!(result.is_err());
    }

    #[test]
    fn test_response_serialize() {
        let response = SendLineRefResponse {
            success:   true,
            reference: "@src/main.rs#L10".to_string(),
        };
        let json = serde_json::to_value(response).unwrap();

        assert_eq!(json["success"], json!(true));
        assert_eq!(json["reference"], json!("@src/main.rs#L10"));
    }

    #[test]
    fn test_reference_format() {
        let file_path = "src/lib.rs";
        let line = 42;

        let expected = "@src/lib.rs#L42";
        let formatted = format!("@{}#L{}", file_path, line);
        assert_eq!(formatted, expected);
    }

    #[test]
    fn test_reference_format_large_line() {
        let file_path = "src/main.rs";
        let line = 9999;

        let expected = "@src/main.rs#L9999";
        let formatted = format!("@{}#L{}", file_path, line);
        assert_eq!(formatted, expected);
    }
}
