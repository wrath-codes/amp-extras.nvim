//! Server status command
//!
//! Returns WebSocket server status information including:
//! - running: Whether server is active
//! - port: Port number (null if not running)
//! - clients: Number of connected clients

use serde_json::{json, Value};

use crate::{errors::Result, server};

/// Get WebSocket server status
///
/// Returns a JSON object with server state:
/// ```json
/// {
///   "running": true,
///   "port": 54321,
///   "clients": 2
/// }
/// ```
///
/// # Arguments
/// * `_args` - Unused (for command signature compatibility)
///
/// # Returns
/// Server status as JSON Value
pub fn server_status(_args: Value) -> Result<Value> {
    let running = server::is_running();
    let port = server::get_port();
    let clients = server::get_hub().map(|h| h.client_count()).unwrap_or(0);

    Ok(json!({
        "running": running,
        "port": port,
        "clients": clients,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_status_when_stopped() {
        // Ensure server is stopped
        crate::server::stop();
        std::thread::sleep(std::time::Duration::from_millis(50));

        let result = server_status(json!({}));
        assert!(result.is_ok());

        let status = result.unwrap();
        assert_eq!(status["running"], json!(false));
        assert!(status["port"].is_null());
        assert_eq!(status["clients"], json!(0));
    }

    #[test]
    fn test_server_status_with_empty_args() {
        crate::server::stop();
        std::thread::sleep(std::time::Duration::from_millis(50));

        let result = server_status(json!({}));
        assert!(result.is_ok());
    }

    #[test]
    fn test_server_status_with_null_args() {
        crate::server::stop();
        std::thread::sleep(std::time::Duration::from_millis(50));

        let result = server_status(json!(null));
        assert!(result.is_ok());
    }

    // Note: Test for running server requires Neovim context (AsyncHandle)
    // Covered by integration tests
}
