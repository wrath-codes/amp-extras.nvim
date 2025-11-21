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
    DatabaseError(#[from] sqlx::Error),

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
            AmpError::ConversionError(_) => "conversion",
            AmpError::Other(_) => "other",
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
}
