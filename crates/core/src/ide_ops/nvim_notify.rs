//! Neovim notification operation

use serde::Deserialize;
use serde_json::Value;

use crate::errors::{AmpError, Result};

/// Parameters for nvim/notify
#[derive(Debug, Deserialize)]
struct NotifyParams {
    message: String,
}

/// Handle nvim/notify request
///
/// Sends a notification to Neovim using AsyncHandle for cross-thread safety.
/// The message is queued and processed on Neovim's main event loop.
///
/// Request:
/// ```json
/// { "message": "Hello from Amp!" }
/// ```
///
/// Response:
/// ```json
/// null
/// ```
///
/// # Errors
/// - InvalidArgs: Missing or invalid message parameter
/// - Other: AsyncHandle not initialized (server not started)
pub fn nvim_notify(params: Value) -> Result<()> {
    let params: NotifyParams =
        serde_json::from_value(params).map_err(|e| AmpError::InvalidArgs {
            command: "nvim/notify".to_string(),
            reason:  e.to_string(),
        })?;

    // Get channel sender
    let tx = super::get_nvim_tx()
        .ok_or_else(|| AmpError::Other("AsyncHandle not initialized".into()))?;

    // Get AsyncHandle to trigger callback
    let async_handle = super::get_async_handle()
        .ok_or_else(|| AmpError::Other("AsyncHandle not initialized".into()))?;

    // Send message through channel
    let msg = super::NvimMessage {
        message: params.message,
    };

    tx.send(msg)
        .map_err(|e| AmpError::Other(format!("Failed to send message: {}", e)))?;

    // Trigger AsyncHandle to process the message
    async_handle
        .send()
        .map_err(|e| AmpError::Other(format!("Failed to trigger AsyncHandle: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

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
            },
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
            },
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
            },
            _ => panic!("Expected InvalidArgs"),
        }
    }
}
