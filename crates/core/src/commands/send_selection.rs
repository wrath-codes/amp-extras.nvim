//! Send selected text to Amp prompt
//!
//! Uses nvim-oxi to get buffer lines from visual selection and sends to Amp
//! prompt.

use nvim_oxi::api::Buffer;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    errors::{AmpError, Result},
    notifications,
};

/// Parameters for send_selection command
#[derive(Debug, Deserialize)]
pub struct SendSelectionParams {
    /// Start line (1-indexed)
    pub start_line: usize,
    /// End line (1-indexed)
    pub end_line:   usize,
}

/// Response for send_selection command
#[derive(Debug, Serialize)]
pub struct SendSelectionResponse {
    /// Success flag
    pub success: bool,
}

/// Send selected text to Amp prompt
///
/// Uses nvim-oxi to get buffer lines from the visual selection and sends
/// them to the Amp prompt field via the `appendToPrompt` notification.
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
///   "success": true
/// }
/// ```
///
/// # Errors
/// - Returns error if WebSocket server is not running
/// - Returns error if buffer access fails
/// - Returns error if notification fails to send
pub fn send_selection(params: Value) -> Result<Value> {
    let params: SendSelectionParams =
        serde_json::from_value(params).map_err(|e| AmpError::InvalidArgs {
            command: "send_selection".to_string(),
            reason:  e.to_string(),
        })?;

    // Get current buffer
    let buf = Buffer::current();

    // Get lines from buffer (convert 1-indexed to 0-indexed, use range)
    let lines = buf
        .get_lines((params.start_line - 1)..params.end_line, false)
        .map_err(|e| AmpError::Other(format!("Failed to get buffer lines: {}", e)))?;

    // Convert nvim_oxi::String iterator to Vec<String>
    // Use to_str() to preserve original UTF-8 without replacement characters
    let content = lines
        .map(|s| {
            s.to_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|_| String::new())
        })
        .collect::<Vec<String>>()
        .join("\n");

    // Get Hub from server
    let hub = crate::server::get_hub()
        .ok_or_else(|| AmpError::Other("WebSocket server not running".into()))?;

    // Send content to prompt via appendToPrompt notification
    notifications::send_append_to_prompt(&hub, &content)?;

    Ok(json!(SendSelectionResponse { success: true }))
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

        let parsed: SendSelectionParams = serde_json::from_value(params).unwrap();
        assert_eq!(parsed.start_line, 10);
        assert_eq!(parsed.end_line, 20);
    }

    #[test]
    fn test_params_deserialize_single_line() {
        let params = json!({
            "start_line": 5,
            "end_line": 5
        });

        let parsed: SendSelectionParams = serde_json::from_value(params).unwrap();
        assert_eq!(parsed.start_line, 5);
        assert_eq!(parsed.end_line, 5);
    }

    #[test]
    fn test_params_missing_fields() {
        let params = json!({
            "start_line": 10
        });

        let result: std::result::Result<SendSelectionParams, _> = serde_json::from_value(params);
        assert!(result.is_err());
    }

    #[test]
    #[ignore = "Requires Neovim context"]
    fn test_send_selection_without_server() {
        // Ensure server is stopped
        crate::server::stop();

        let params = json!({
            "start_line": 1,
            "end_line": 5
        });

        let result = send_selection(params);
        assert!(result.is_err());
    }

    #[test]
    fn test_response_serialize() {
        let response = SendSelectionResponse { success: true };
        let json = serde_json::to_value(response).unwrap();

        assert_eq!(json["success"], json!(true));
    }
}
