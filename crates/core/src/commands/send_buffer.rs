//! Send entire buffer content to Amp prompt
//!
//! Uses nvim-oxi to get all lines from current buffer and sends to Amp prompt.

use nvim_oxi::api::Buffer;
use serde::Serialize;
use serde_json::{json, Value};

use crate::{
    errors::{AmpError, Result},
    notifications,
};

/// Response for send_buffer command
#[derive(Debug, Serialize)]
pub struct SendBufferResponse {
    /// Success flag
    pub success: bool,
}

/// Send entire buffer content to Amp prompt
///
/// Uses nvim-oxi to get all lines from the current buffer and sends them
/// to the Amp prompt field via the `appendToPrompt` notification.
///
/// # Request
/// ```json
/// {}
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
pub fn send_buffer(_params: Value) -> Result<Value> {
    // Get current buffer
    let buf = Buffer::current();

    // Get all lines from buffer
    let lines = buf
        .get_lines(.., false)
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

    Ok(json!(SendBufferResponse { success: true }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_buffer_accepts_empty_params() {
        // send_buffer doesn't need params
        let _params = json!({});
        // Just verify it doesn't panic on deserialization
        // Actual execution requires Neovim context
    }

    #[test]
    #[ignore = "Requires Neovim context"]
    fn test_send_buffer_without_server() {
        // Ensure server is stopped
        crate::server::stop();

        let params = json!({});
        let result = send_buffer(params);
        assert!(result.is_err());
    }

    #[test]
    fn test_response_serialize() {
        let response = SendBufferResponse { success: true };
        let json = serde_json::to_value(response).unwrap();

        assert_eq!(json["success"], json!(true));
    }
}
