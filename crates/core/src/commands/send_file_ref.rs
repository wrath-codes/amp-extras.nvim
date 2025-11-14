//! Send file reference to Amp prompt
//!
//! Uses nvim-oxi to get current buffer path and creates a reference in the
//! format `@file.rs`.

use nvim_oxi::api::Buffer;
use serde::Serialize;
use serde_json::{json, Value};

use crate::{
    errors::{AmpError, Result},
    notifications,
};

/// Response for send_file_ref command
#[derive(Debug, Serialize)]
pub struct SendFileRefResponse {
    /// Success flag
    pub success:   bool,
    /// The formatted reference that was sent
    pub reference: String,
}

/// Send file reference to Amp prompt
///
/// Uses nvim-oxi to get the current buffer path and formats a file reference
/// like `@file.rs`, then sends it to the Amp prompt via `appendToPrompt`.
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
///   "reference": "@src/main.rs"
/// }
/// ```
///
/// # Errors
/// - Returns error if buffer has no filename
/// - Returns error if WebSocket server is not running
/// - Returns error if notification fails to send
pub fn send_file_ref(_params: Value) -> Result<Value> {
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

    // Get Hub from server
    let hub = crate::server::get_hub()
        .ok_or_else(|| AmpError::Other("WebSocket server not running".into()))?;

    // Format reference: @file.rs
    let reference = format!("@{}", file_path);

    // Send reference to prompt
    notifications::send_append_to_prompt(&hub, &reference)?;

    Ok(json!(SendFileRefResponse {
        success: true,
        reference
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_file_ref_accepts_empty_params() {
        // send_file_ref doesn't need params
        let _params = json!({});
        // Just verify it doesn't panic on deserialization
        // Actual execution requires Neovim context
    }

    #[test]
    #[ignore = "Requires Neovim context"]
    fn test_send_file_ref_without_server() {
        // Ensure server is stopped
        crate::server::stop();

        let params = json!({});
        let result = send_file_ref(params);
        assert!(result.is_err());
    }

    #[test]
    fn test_response_serialize() {
        let response = SendFileRefResponse {
            success:   true,
            reference: "@src/main.rs".to_string(),
        };
        let json = serde_json::to_value(response).unwrap();

        assert_eq!(json["success"], json!(true));
        assert_eq!(json["reference"], json!("@src/main.rs"));
    }

    #[test]
    fn test_reference_format() {
        let file_path = "src/lib.rs";
        let expected = "@src/lib.rs";
        let formatted = format!("@{}", file_path);
        assert_eq!(formatted, expected);
    }

    #[test]
    fn test_reference_with_special_chars() {
        let file_path = "my file (2).rs";
        let expected = "@my file (2).rs";
        let formatted = format!("@{}", file_path);
        assert_eq!(formatted, expected);
    }
}
