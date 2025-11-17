//! Error types for amp-extras-rs
//!
//! This module defines error types that bridge Rust and Lua, ensuring
//! error messages are user-friendly across the FFI boundary.

use thiserror::Error;

/// Result type alias for amp-extras operations
pub type Result<T> = std::result::Result<T, AmpError>;

/// Main error type for amp-extras
#[derive(Debug, Error)]
pub enum AmpError {
    /// Command not found in registry
    #[error("Command not found: {0}")]
    CommandNotFound(String),

    /// Invalid command arguments
    #[error("Invalid arguments for command '{command}': {reason}")]
    InvalidArgs { command: String, reason: String },

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    SerdeError(#[from] serde_json::Error),

    /// Database error
    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Amp CLI execution error
    #[error("Amp CLI error: {0}")]
    AmpCliError(String),

    /// Thread parsing error
    #[error("Failed to parse thread file: {0}")]
    ThreadParseError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Validation error
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// WebSocket error
    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    /// URL parsing error
    #[error("URL parsing error: {0}")]
    UrlParseError(String),

    /// Hub error (client messaging)
    #[error("Hub error: {0}")]
    HubError(String),

    /// Notification error (broadcast/serialization)
    #[error("Notification error: {0}")]
    NotificationError(String),

    /// Conversion error (nvim-oxi Object ↔ Rust types)
    #[error("Conversion error: {0}")]
    ConversionError(String),

    /// Generic error (catch-all)
    #[error("{0}")]
    Other(String),
}

impl From<anyhow::Error> for AmpError {
    fn from(err: anyhow::Error) -> Self {
        AmpError::Other(err.to_string())
    }
}

impl From<String> for AmpError {
    fn from(err: String) -> Self {
        AmpError::Other(err)
    }
}

impl From<&str> for AmpError {
    fn from(err: &str) -> Self {
        AmpError::Other(err.to_string())
    }
}

impl From<tungstenite::Error> for AmpError {
    fn from(err: tungstenite::Error) -> Self {
        AmpError::WebSocketError(err.to_string())
    }
}

impl From<url::ParseError> for AmpError {
    fn from(err: url::ParseError) -> Self {
        AmpError::UrlParseError(err.to_string())
    }
}

/// Convert AmpError to a Lua-friendly error message
impl AmpError {
    /// Get user-friendly error message for display in Neovim
    pub fn user_message(&self) -> String {
        match self {
            AmpError::CommandNotFound(cmd) => {
                format!(
                    "Command '{}' not found. Run :AmpHelp for available commands.",
                    cmd
                )
            },
            AmpError::InvalidArgs { command, reason } => {
                format!("Invalid arguments for '{}': {}", command, reason)
            },
            AmpError::AmpCliError(msg) => {
                format!("Amp CLI error: {}", msg)
            },
            AmpError::DatabaseError(err) => {
                format!("Database error: {}", err)
            },
            _ => self.to_string(),
        }
    }

    /// Get error category for logging/telemetry
    pub fn category(&self) -> &'static str {
        match self {
            AmpError::CommandNotFound(_) => "command",
            AmpError::InvalidArgs { .. } => "arguments",
            AmpError::SerdeError(_) => "serialization",
            AmpError::DatabaseError(_) => "database",
            AmpError::IoError(_) => "io",
            AmpError::AmpCliError(_) => "amp_cli",
            AmpError::ThreadParseError(_) => "thread_parse",
            AmpError::ConfigError(_) => "config",
            AmpError::ValidationError(_) => "validation",
            AmpError::WebSocketError(_) => "websocket",
            AmpError::UrlParseError(_) => "url_parse",
            AmpError::HubError(_) => "hub",
            AmpError::NotificationError(_) => "notification",
            AmpError::ConversionError(_) => "conversion",
            AmpError::Other(_) => "other",
        }
    }

    /// Convert AmpError to JSON-RPC error code
    ///
    /// Maps internal errors to standard JSON-RPC 2.0 error codes:
    /// - -32700: Parse error (invalid JSON)
    /// - -32600: Invalid request
    /// - -32601: Method not found
    /// - -32602: Invalid params
    /// - -32603: Internal error
    /// - -32000 to -32099: Server errors (custom)
    pub fn to_jsonrpc_code(&self) -> i32 {
        match self {
            AmpError::SerdeError(_) => -32700,        // Parse error
            AmpError::InvalidArgs { .. } => -32602,   // Invalid params
            AmpError::CommandNotFound(_) => -32601,   // Method not found
            AmpError::ValidationError(_) => -32600,   // Invalid request
            AmpError::WebSocketError(_) => -32001,    // Server error (WebSocket)
            AmpError::UrlParseError(_) => -32600,     // Invalid request
            AmpError::DatabaseError(_) => -32002,     // Server error (Database)
            AmpError::IoError(_) => -32003,           // Server error (I/O)
            AmpError::AmpCliError(_) => -32004,       // Server error (CLI)
            AmpError::ThreadParseError(_) => -32700,  // Parse error
            AmpError::ConfigError(_) => -32005,       // Server error (Config)
            AmpError::HubError(_) => -32006,          // Server error (Hub)
            AmpError::NotificationError(_) => -32007, // Server error (Notification)
            AmpError::ConversionError(_) => -32008,   // Server error (Conversion)
            AmpError::Other(_) => -32603,             // Internal error
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = AmpError::CommandNotFound("test.command".to_string());
        assert_eq!(err.to_string(), "Command not found: test.command");
    }

    #[test]
    fn test_user_message() {
        let err = AmpError::CommandNotFound("test.command".to_string());
        assert!(err.user_message().contains("test.command"));
        assert!(err.user_message().contains("AmpHelp"));
    }

    #[test]
    fn test_error_category() {
        assert_eq!(
            AmpError::CommandNotFound("test".to_string()).category(),
            "command"
        );
        assert_eq!(
            AmpError::InvalidArgs {
                command: "test".to_string(),
                reason:  "bad".to_string(),
            }
            .category(),
            "arguments"
        );
    }

    #[test]
    fn test_from_string() {
        let err: AmpError = "test error".into();
        assert_eq!(err.to_string(), "test error");
    }

    #[test]
    fn test_websocket_error_category() {
        let err = AmpError::WebSocketError("connection failed".to_string());
        assert_eq!(err.category(), "websocket");
    }

    #[test]
    fn test_url_parse_error_category() {
        let err = AmpError::UrlParseError("invalid url".to_string());
        assert_eq!(err.category(), "url_parse");
    }

    #[test]
    fn test_hub_error_category() {
        let err = AmpError::HubError("client not found".to_string());
        assert_eq!(err.category(), "hub");
    }

    #[test]
    fn test_notification_error_category() {
        let err = AmpError::NotificationError("serialization failed".to_string());
        assert_eq!(err.category(), "notification");
    }

    #[test]
    fn test_from_tungstenite_error() {
        // Create a tungstenite error by trying to parse invalid data
        let ws_err = tungstenite::Error::Protocol(
            tungstenite::error::ProtocolError::ResetWithoutClosingHandshake,
        );
        let amp_err: AmpError = ws_err.into();

        match amp_err {
            AmpError::WebSocketError(_) => {
                assert_eq!(amp_err.category(), "websocket");
            },
            _ => panic!("Expected WebSocketError"),
        }
    }

    #[test]
    fn test_from_url_parse_error() {
        let url_err = url::Url::parse("not a valid url").unwrap_err();
        let amp_err: AmpError = url_err.into();

        match amp_err {
            AmpError::UrlParseError(_) => {
                assert_eq!(amp_err.category(), "url_parse");
            },
            _ => panic!("Expected UrlParseError"),
        }
    }

    #[test]
    fn test_jsonrpc_error_codes() {
        // Standard JSON-RPC error codes
        assert_eq!(
            AmpError::SerdeError(serde_json::from_str::<i32>("invalid").unwrap_err())
                .to_jsonrpc_code(),
            -32700 // Parse error
        );
        assert_eq!(
            AmpError::CommandNotFound("test".to_string()).to_jsonrpc_code(),
            -32601 // Method not found
        );
        assert_eq!(
            AmpError::InvalidArgs {
                command: "test".to_string(),
                reason:  "bad".to_string(),
            }
            .to_jsonrpc_code(),
            -32602 // Invalid params
        );
        assert_eq!(
            AmpError::ValidationError("invalid".to_string()).to_jsonrpc_code(),
            -32600 // Invalid request
        );

        // Custom server error codes
        assert_eq!(
            AmpError::WebSocketError("error".to_string()).to_jsonrpc_code(),
            -32001
        );
        assert_eq!(
            AmpError::DatabaseError(rusqlite::Error::InvalidQuery).to_jsonrpc_code(),
            -32002
        );
        assert_eq!(
            AmpError::IoError(std::io::Error::new(std::io::ErrorKind::NotFound, "test"))
                .to_jsonrpc_code(),
            -32003
        );
        assert_eq!(
            AmpError::AmpCliError("error".to_string()).to_jsonrpc_code(),
            -32004
        );
        assert_eq!(
            AmpError::ConfigError("error".to_string()).to_jsonrpc_code(),
            -32005
        );
        assert_eq!(
            AmpError::HubError("error".to_string()).to_jsonrpc_code(),
            -32006
        );
        assert_eq!(
            AmpError::NotificationError("error".to_string()).to_jsonrpc_code(),
            -32007
        );
        assert_eq!(
            AmpError::Other("error".to_string()).to_jsonrpc_code(),
            -32603 // Internal error
        );
    }
}
