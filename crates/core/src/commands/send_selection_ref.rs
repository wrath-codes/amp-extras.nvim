//! Send file reference with line range to Amp prompt
//!
//! Uses nvim-oxi to get current buffer path and creates a reference in the
//! format `@file.rs#L10-L20`.

use nvim_oxi::{api::Buffer, string};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    errors::{AmpError, Result},
    notifications,
};

/// Parameters for send_selection_ref command
#[derive(Debug, Deserialize)]
pub struct SendSelectionRefParams {
    /// Selection start line (1-indexed)
    pub start_line: usize,
    /// Selection end line (1-indexed)
    pub end_line:   usize,
}

/// Response for send_selection_ref command
#[derive(Debug, Serialize)]
pub struct SendSelectionRefResponse {
    /// Success flag
    pub success:   bool,
    /// The formatted reference that was sent
    pub reference: String,
}

/// Send file reference with line range to Amp prompt
///
/// Uses nvim-oxi to get the current buffer path and formats a file reference
/// like `@file.rs#L10-L20`, then sends it to the Amp prompt via
/// `appendToPrompt`.
///
/// # Request
/// ```json
/// {
///   "start_line": 10,
///   "end_line": 20
/// }
/// ```
///
/// # Response
/// ```json
/// {
///   "success": true,
///   "reference": "@src/main.rs#L10-L20"
/// }
/// ```
///
/// # Errors
/// - Returns error if buffer has no filename
/// - Returns error if WebSocket server is not running
/// - Returns error if notification fails to send
pub fn send_selection_ref(params: Value) -> Result<Value> {
    let params: SendSelectionRefParams =
        serde_json::from_value(params).map_err(|e| AmpError::InvalidArgs {
            command: "send_selection_ref".to_string(),
            reason:  e.to_string(),
        })?;

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

    // Format reference: @file.rs#L10-L20
    let reference = if params.start_line == params.end_line {
        // Single line: @file.rs#L10
        string!("@{}#L{}", file_path, params.start_line).to_string()
    } else {
        // Range: @file.rs#L10-L20
        string!("@{}#L{}-L{}", file_path, params.start_line, params.end_line).to_string()
    };

    // Send reference to prompt
    notifications::send_append_to_prompt(&hub, &reference)?;

    Ok(json!(SendSelectionRefResponse {
        success: true,
        reference
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_params_deserialize() {
        let params = json!({
            "start_line": 10,
            "end_line": 20
        });

        let parsed: SendSelectionRefParams = serde_json::from_value(params).unwrap();
        assert_eq!(parsed.start_line, 10);
        assert_eq!(parsed.end_line, 20);
    }

    #[test]
    fn test_params_deserialize_single_line() {
        let params = json!({
            "start_line": 5,
            "end_line": 5
        });

        let parsed: SendSelectionRefParams = serde_json::from_value(params).unwrap();
        assert_eq!(parsed.start_line, 5);
        assert_eq!(parsed.end_line, 5);
    }

    #[test]
    fn test_params_missing_fields() {
        let params = json!({
            "start_line": 10
        });

        let result: std::result::Result<SendSelectionRefParams, _> = serde_json::from_value(params);
        assert!(result.is_err());
    }

    #[test]
    #[ignore = "Requires Neovim context"]
    fn test_send_selection_ref_without_server() {
        // Ensure server is stopped
        crate::server::stop();

        let params = json!({
            "start_line": 10,
            "end_line": 20
        });

        let result = send_selection_ref(params);
        assert!(result.is_err());
    }

    #[test]
    fn test_response_serialize() {
        let response = SendSelectionRefResponse {
            success:   true,
            reference: "@src/main.rs#L10-L20".to_string(),
        };
        let json = serde_json::to_value(response).unwrap();

        assert_eq!(json["success"], json!(true));
        assert_eq!(json["reference"], json!("@src/main.rs#L10-L20"));
    }

    #[test]
    fn test_reference_format_range() {
        // Test the formatting logic directly
        let file_path = "src/main.rs";
        let start_line = 10;
        let end_line = 20;

        let expected = "@src/main.rs#L10-L20";
        let formatted = format!("@{}#L{}-L{}", file_path, start_line, end_line);
        assert_eq!(formatted, expected);
    }

    #[test]
    fn test_reference_format_single_line() {
        let file_path = "src/lib.rs";
        let line = 5;

        let expected = "@src/lib.rs#L5";
        let formatted = format!("@{}#L{}", file_path, line);
        assert_eq!(formatted, expected);
    }
}
