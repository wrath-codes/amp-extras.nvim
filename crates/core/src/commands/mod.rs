//! Command registry and dispatch system
//!
//! This module provides a static registry of commands that can be called from Lua.
//! Commands are registered as "category.action" (e.g., "threads.list", "prompts.create")
//! and dispatched to handler functions.
//!
//! ## Adding a new command
//!
//! 1. Create handler function: `pub fn my_command(args: Value) -> Result<Value>`
//! 2. Register in `REGISTRY`: `("category.action", my_command as CommandHandler)`
//! 3. Add tests for the command

use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;

use crate::errors::{AmpError, Result};

/// Type alias for command handler functions
///
/// All command handlers take a JSON Value (arguments) and return a Result<Value>
pub type CommandHandler = fn(Value) -> Result<Value>;

/// Static command registry
///
/// Maps command names to handler functions. Initialized lazily on first access.
static REGISTRY: Lazy<HashMap<&'static str, CommandHandler>> = Lazy::new(|| {
    let mut map = HashMap::new();

    // Test command
    map.insert("ping", ping as CommandHandler);

    map
});

/// Dispatch a command by name
///
/// Looks up the command in the registry and executes it with the provided arguments.
///
/// # Arguments
/// * `command` - Command name (e.g., "ping", "threads.list")
/// * `args` - Command arguments as JSON Value
///
/// # Returns
/// Command result as JSON Value, or error if command not found
pub fn dispatch(command: &str, args: Value) -> Result<Value> {
    match REGISTRY.get(command) {
        Some(handler) => handler(args),
        None => Err(AmpError::CommandNotFound(command.to_string())),
    }
}

/// List all available commands
///
/// Returns a sorted list of all registered command names.
pub fn list_commands() -> Vec<String> {
    let mut commands: Vec<String> = REGISTRY.keys().map(|&k| k.to_string()).collect();
    commands.sort();
    commands
}

// ============================================================================
// Test Commands
// ============================================================================

/// Ping command - simple test to verify command dispatch works
///
/// Returns the input arguments with an added "pong" field.
///
/// # Example
/// ```json
/// // Input:  {"message": "hello"}
/// // Output: {"message": "hello", "pong": true}
/// ```
fn ping(args: Value) -> Result<Value> {
    let mut result = match args {
        Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };

    result.insert("pong".to_string(), Value::Bool(true));
    Ok(Value::Object(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ========================================
    // dispatch() tests
    // ========================================

    #[test]
    fn test_dispatch_ping() {
        let args = json!({"message": "hello"});
        let result = dispatch("ping", args);

        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["pong"], json!(true));
        assert_eq!(value["message"], json!("hello"));
    }

    #[test]
    fn test_dispatch_unknown_command() {
        let args = json!({});
        let result = dispatch("unknown.command", args);

        assert!(result.is_err());
        match result {
            Err(AmpError::CommandNotFound(cmd)) => {
                assert_eq!(cmd, "unknown.command");
            }
            _ => panic!("Expected CommandNotFound error"),
        }
    }

    #[test]
    fn test_dispatch_with_empty_args() {
        let args = json!({});
        let result = dispatch("ping", args);

        assert!(result.is_ok());
        assert_eq!(result.unwrap()["pong"], json!(true));
    }

    #[test]
    fn test_dispatch_with_null_args() {
        let args = json!(null);
        let result = dispatch("ping", args);

        assert!(result.is_ok());
        assert_eq!(result.unwrap()["pong"], json!(true));
    }

    // ========================================
    // list_commands() tests
    // ========================================

    #[test]
    fn test_list_commands_includes_ping() {
        let commands = list_commands();
        assert!(commands.contains(&"ping".to_string()));
    }

    #[test]
    fn test_list_commands_is_sorted() {
        let commands = list_commands();
        let mut sorted = commands.clone();
        sorted.sort();
        assert_eq!(commands, sorted);
    }

    #[test]
    fn test_list_commands_not_empty() {
        let commands = list_commands();
        assert!(!commands.is_empty());
    }

    // ========================================
    // ping command tests
    // ========================================

    #[test]
    fn test_ping_adds_pong_field() {
        let args = json!({"test": "value"});
        let result = ping(args).unwrap();

        assert_eq!(result["pong"], json!(true));
        assert_eq!(result["test"], json!("value"));
    }

    #[test]
    fn test_ping_with_empty_object() {
        let args = json!({});
        let result = ping(args).unwrap();

        assert_eq!(result["pong"], json!(true));
    }

    #[test]
    fn test_ping_with_non_object() {
        // ping should handle non-object input gracefully
        let args = json!(42);
        let result = ping(args).unwrap();

        assert_eq!(result["pong"], json!(true));
    }

    #[test]
    fn test_ping_preserves_fields() {
        let args = json!({
            "field1": "value1",
            "field2": 42,
            "field3": true
        });
        let result = ping(args).unwrap();

        assert_eq!(result["pong"], json!(true));
        assert_eq!(result["field1"], json!("value1"));
        assert_eq!(result["field2"], json!(42));
        assert_eq!(result["field3"], json!(true));
    }
}
