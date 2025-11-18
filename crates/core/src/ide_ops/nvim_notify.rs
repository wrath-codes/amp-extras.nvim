//! Neovim notification operation

use nvim_oxi::print;
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
/// Sends a notification to Neovim using schedule() for cross-thread safety.
pub fn nvim_notify(params: Value) -> Result<()> {
    let params: NotifyParams =
        serde_json::from_value(params).map_err(|e| AmpError::InvalidArgs {
            command: "nvim/notify".to_string(),
            reason:  e.to_string(),
        })?;

    // Use nvim-oxi's schedule() to safely print from any thread
    #[cfg(not(test))]
    {
        nvim_oxi::schedule(move |_| {
            print!("{}", params.message);
            Ok::<_, std::convert::Infallible>(())
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_nvim_notify_parses_params() {
        // Should succeed in parsing (can't test actual print in unit test)
        let result = nvim_notify(json!({
            "message": "Test notification"
        }));

        assert!(result.is_ok());
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
